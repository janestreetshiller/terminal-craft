//! Native QA drives the real map picker and unmodified bundled layouts.
use super::{buttons, err};
use crate::{
    game::Game,
    maps,
    world::{World, SX, SY, SZ},
};
use sdl2::{
    event::Event,
    keyboard::{Keycode, Mod},
    mouse::{MouseButton, MouseState},
};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub(super) struct PackSmoke {
    dir: PathBuf,
    start: [f32; 5],
    saved: [f32; 5],
    walk: f32,
    jump: bool,
    edited: bool,
    reports: Vec<String>,
}
impl PackSmoke {
    pub(super) fn new(dir: PathBuf) -> io::Result<Self> {
        if dir.exists() {
            return Err(err(
                "Map QA requires a fresh output directory; normal saves are never used",
            ));
        }
        fs::create_dir_all(&dir)?;
        fs::write(
            dir.join("world.tcrf"),
            b"existing world must remain untouched",
        )?;
        Ok(Self {
            dir,
            start: [0.0; 5],
            saved: [0.0; 5],
            walk: 0.0,
            jump: false,
            edited: false,
            reports: Vec::new(),
        })
    }
    pub(super) fn inject(&self, n: u32, events: &sdl2::EventSubsystem, id: u32) -> io::Result<()> {
        let push = |e| events.push_event(e).map_err(err);
        let send = |k, down| {
            push(if down {
                Event::KeyDown {
                    timestamp: 0,
                    window_id: id,
                    keycode: Some(k),
                    scancode: None,
                    keymod: Mod::NOMOD,
                    repeat: false,
                }
            } else {
                Event::KeyUp {
                    timestamp: 0,
                    window_id: id,
                    keycode: Some(k),
                    scancode: None,
                    keymod: Mod::NOMOD,
                    repeat: false,
                }
            })
        };
        let click = |button, x, y, down| {
            push(if down {
                Event::MouseButtonDown {
                    timestamp: 0,
                    window_id: id,
                    which: 0x5443,
                    mouse_btn: button,
                    clicks: 1,
                    x,
                    y,
                }
            } else {
                Event::MouseButtonUp {
                    timestamp: 0,
                    window_id: id,
                    which: 0x5443,
                    mouse_btn: button,
                    clicks: 1,
                    x,
                    y,
                }
            })
        };
        let motion = |dy| {
            push(Event::MouseMotion {
                timestamp: 0,
                window_id: id,
                which: 0x5443,
                mousestate: MouseState::from_sdl_state(0),
                x: 550,
                y: 360,
                xrel: 0,
                yrel: dy,
            })
        };
        if n == 400 {
            return push(Event::Quit { timestamp: 0 });
        }
        let index = (n / 80) as usize;
        match n % 80 {
            1 | 62 => {
                let r = buttons(1100, 720, 5)[3];
                click(MouseButton::Left, r.x + r.w / 2, r.y + r.h / 2, true)?;
                click(MouseButton::Left, r.x + r.w / 2, r.y + r.h / 2, false)?;
            }
            3 | 63 => send(
                [
                    Keycode::Num1,
                    Keycode::Num2,
                    Keycode::Num3,
                    Keycode::Num4,
                    Keycode::Num5,
                ][index],
                true,
            )?,
            5 => send(Keycode::W, true)?,
            7 => send(Keycode::Space, true)?,
            8 => send(Keycode::Space, false)?,
            30 => send(Keycode::W, false)?,
            32 => motion(450)?,
            34 => {
                send(Keycode::Num1, true)?;
                click(MouseButton::Right, 550, 360, true)?;
            }
            35 => click(MouseButton::Right, 550, 360, false)?,
            38 => motion(-450)?,
            54 | 56 => send(Keycode::M, true)?,
            58 | 65 => send(Keycode::Escape, true)?,
            60 | 66 => {
                send(Keycode::Down, true)?;
                send(Keycode::Return, true)?;
            }
            _ => {}
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn observe(
        &mut self,
        n: u32,
        game: Option<&Game>,
        base: &Path,
        rgba: &[u8],
        w: u32,
        h: u32,
        captured: bool,
    ) -> io::Result<()> {
        let map = &maps::MAPS[(n / 80) as usize];
        let phase = n % 80;
        let path = maps::save_path(base, map.id)?;
        let body = 28..28 + (SX * SY * SZ) as usize;
        let current =
            || game.ok_or_else(|| err(format!("{}: map did not open at frame {n}", map.id)));
        match phase {
            4 => {
                self.start = current()?.snapshot();
                self.walk = 0.0;
                self.jump = false;
                self.edited = false;
                if !captured || fs::read(&path)?[body.clone()] != map.bytes[body.clone()] {
                    return Err(err(format!(
                        "{}: pointer lock or pristine layout failed",
                        map.id
                    )));
                }
            }
            14 => self.jump = current()?.snapshot()[1] > self.start[1] + 0.3,
            31 => {
                let pose = current()?.snapshot();
                self.walk =
                    ((pose[0] - self.start[0]).powi(2) + (pose[2] - self.start[2]).powi(2)).sqrt();
            }
            36 => {
                current()?.save()?;
                let data = fs::read(&path)?;
                let changes: Vec<_> = data[body.clone()]
                    .iter()
                    .zip(&map.bytes[body.clone()])
                    .filter(|(a, b)| a != b)
                    .collect();
                self.edited = changes.len() == 1 && changes[0].0 != &0 && changes[0].1 == &0;
            }
            61 => {
                if game.is_some() || captured {
                    return Err(err(format!("{}: save-and-main-menu failed", map.id)));
                }
                self.saved = World::load(&path)?
                    .4
                    .ok_or_else(|| err("missing player state"))?
                    .pose;
            }
            64 => {
                let pose = current()?.snapshot();
                let reloaded = pose
                    .iter()
                    .zip(self.saved)
                    .all(|(a, b)| (*a - b).abs() < 0.00001);
                current()?.save()?;
                let data = fs::read(&path)?;
                let edits_retained = data[body.clone()]
                    .iter()
                    .zip(&map.bytes[body])
                    .filter(|(a, b)| a != b)
                    .count()
                    == 1;
                let passed = self.walk > 1.0
                    && self.jump
                    && self.edited
                    && reloaded
                    && edits_retained
                    && captured;
                self.reports.push(format!("{{\"id\":\"{}\",\"passed\":{passed},\"walk_distance\":{},\"jump\":{},\"placed_one_block\":{},\"save_reload_pose\":{reloaded},\"edit_retained\":{edits_retained},\"pointer_locked\":{captured}}}",map.id,self.walk,self.jump,self.edited));
                fs::write(
                    self.dir.join("maps.json"),
                    format!("[{}]\n", self.reports.join(",\n")),
                )?;
                if !passed {
                    return Err(err(format!("{}: map QA failed; see maps.json", map.id)));
                }
            }
            _ => {}
        }
        let name = match phase {
            0 if n == 0 => Some("menu".to_string()),
            1 if n == 1 => Some("picker".to_string()),
            2 if n == 2 => Some("picker-compact".to_string()),
            4 => Some(format!("{}-gameplay", map.id)),
            55 => Some(format!("{}-overhead", map.id)),
            _ => None,
        };
        if let Some(name) = name {
            let mut file = fs::File::create(self.dir.join(format!("{name}.ppm")))?;
            write!(file, "P6\n{w} {h}\n255\n")?;
            let rgb: Vec<u8> = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| p[..3].iter().copied())
                .collect();
            file.write_all(&rgb)?;
        }
        Ok(())
    }
    pub(super) fn finish(self, released: bool) -> io::Result<()> {
        let default_unchanged =
            fs::read(self.dir.join("world.tcrf"))? == b"existing world must remain untouched";
        let passed = self.reports.len() == maps::MAPS.len() && default_unchanged && released;
        let report=format!("{{\"passed\":{passed},\"map_count\":{},\"default_world_unchanged\":{default_unchanged},\"pointer_released_on_exit\":{released},\"maps\":[{}]}}\n",self.reports.len(),self.reports.join(","));
        fs::write(self.dir.join("report.json"), &report)?;
        println!("{report}");
        if passed {
            Ok(())
        } else {
            Err(err("Native map pack QA did not finish all five maps"))
        }
    }
}
