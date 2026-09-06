//! Native SDL host. Simulation, saves, materials and tools stay in the shared Rust engine.
#[path = "pack_smoke.rs"]
mod pack_smoke;
use crate::{
    game::Game,
    maps,
    player::Player,
    render::{self, Frame},
    world::{World, SX, SY, SZ},
};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode, MouseButton, MouseEvent,
    MouseEventKind,
};
use pack_smoke::PackSmoke;
use sdl2::{
    event::{Event as SdlEvent, WindowEvent},
    keyboard::{Keycode, Mod},
    mouse::MouseButton as SdlButton,
    pixels::PixelFormatEnum,
    video::FullscreenType,
};
use std::{
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn err(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}
fn capture_allowed(game: bool, paused: bool, focused: bool) -> bool {
    game && !paused && focused
}
struct ReleasePointer(sdl2::mouse::MouseUtil);
impl Drop for ReleasePointer {
    fn drop(&mut self) {
        self.0.set_relative_mouse_mode(false);
        self.0.show_cursor(true);
    }
}

fn key_event(key: Keycode, mods: Mod, down: bool, repeat: bool) -> Option<Event> {
    let code = match key {
        Keycode::Escape => KeyCode::Esc,
        Keycode::Return | Keycode::KpEnter => KeyCode::Enter,
        Keycode::Space => KeyCode::Char(' '),
        Keycode::Tab => KeyCode::Tab,
        Keycode::Left => KeyCode::Left,
        Keycode::Right => KeyCode::Right,
        Keycode::Up => KeyCode::Up,
        Keycode::Down => KeyCode::Down,
        Keycode::F3 => KeyCode::F(3),
        Keycode::F6 => KeyCode::F(6),
        Keycode::LShift => KeyCode::Modifier(ModifierKeyCode::LeftShift),
        Keycode::RShift => KeyCode::Modifier(ModifierKeyCode::RightShift),
        Keycode::LCtrl => KeyCode::Modifier(ModifierKeyCode::LeftControl),
        Keycode::RCtrl => KeyCode::Modifier(ModifierKeyCode::RightControl),
        _ => {
            let n = i32::from(key);
            if (32..=126).contains(&n) {
                KeyCode::Char((n as u8 as char).to_ascii_lowercase())
            } else {
                return None;
            }
        }
    };
    let mut modifiers = KeyModifiers::NONE;
    if mods.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD) {
        modifiers |= KeyModifiers::SHIFT;
    }
    if mods.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) {
        modifiers |= KeyModifiers::CONTROL;
    }
    if mods.intersects(Mod::LALTMOD | Mod::RALTMOD) {
        modifiers |= KeyModifiers::ALT;
    }
    let kind = if !down {
        KeyEventKind::Release
    } else if repeat {
        KeyEventKind::Repeat
    } else {
        KeyEventKind::Press
    };
    Some(Event::Key(KeyEvent::new_with_kind(code, modifiers, kind)))
}
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}
#[derive(Clone, Copy)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}
fn buttons(w: i32, h: i32, n: usize) -> Vec<Rect> {
    let bw = (w * 2 / 5).clamp(230, 400);
    let bh = (h / 15).clamp(32, 48);
    let gap = 10;
    let y = h / 2 - (n as i32 * (bh + gap) - gap) / 2 + 35;
    (0..n)
        .map(|i| Rect {
            x: (w - bw) / 2,
            y: y + i as i32 * (bh + gap),
            w: bw,
            h: bh,
        })
        .collect()
}
fn hit_button(rects: &[Rect], x: i32, y: i32) -> Option<usize> {
    rects
        .iter()
        .position(|r| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h)
}
fn text_center(p: &mut [u8], w: i32, h: i32, y: i32, text: &str, color: (u8, u8, u8), scale: f32) {
    let paint = if crate::ui::gilded() {
        crate::ui::text
    } else {
        render::blit_text_px
    };
    paint(
        p,
        w,
        h,
        (w - render::text_width_px(text, scale)) / 2,
        y,
        text,
        color,
        scale,
    );
}
fn panel(
    p: &mut [u8],
    w: i32,
    h: i32,
    title: &str,
    labels: &[&str],
    selected: usize,
    can_continue: bool,
) {
    for c in p.as_chunks_mut::<4>().0 {
        c[0] /= 3;
        c[1] /= 3;
        c[2] /= 3;
    }
    let rects = buttons(w, h, labels.len());
    let title_y = rects[0].y - 80;
    if crate::ui::gilded() {
        let first = rects[0];
        let last = rects[rects.len() - 1];
        crate::ui::panel(
            p,
            w,
            h,
            first.x - 14,
            first.y - 14,
            first.w + 28,
            last.y + last.h - first.y + 28,
        );
    }
    text_center(
        p,
        w,
        h,
        title_y,
        title,
        render::TEXT,
        if w < 800 { 2.5 } else { 3.5 },
    );
    for (i, (r, label)) in rects.iter().zip(labels).enumerate() {
        let disabled = title == "TERMINAL CRAFT" && i == 0 && !can_continue;
        let color = if i == selected && !disabled {
            render::AMBER
        } else {
            (60, 73, 71)
        };
        if crate::ui::gilded() {
            crate::ui::frame(p, w, h, r.x, r.y, r.w, r.h, i == selected, disabled);
        } else {
            render::fill_rect(p, w, h, r.x, r.y, r.w, r.h, color);
            render::fill_rect(p, w, h, r.x + 2, r.y + 2, r.w - 4, r.h - 4, (23, 30, 31));
        }
        text_center(
            p,
            w,
            h,
            r.y + (r.h - 14) / 2,
            label,
            if disabled { (84, 94, 94) } else { render::TEXT },
            2.0,
        );
    }
    let hint = if title == "PAUSED" {
        "H CONTROLS   I INVENTORY   M MAP   ESC RESUME"
    } else {
        "ENTER PLAY   F6 STYLE   F11 FULLSCREEN"
    };
    text_center(p, w, h, h - 24, hint, render::DIM, 1.0);
}
pub(crate) fn paint_map(p: &mut [u8], w: i32, h: i32, world: &World, player: &Player) {
    render::fill_rect(p, w, h, 0, 0, w, h, (20, 26, 27));
    let scale = ((w - 60) / SX).min((h - 110) / SZ).max(1);
    let ox = (w - SX * scale) / 2;
    let oy = (h - SZ * scale) / 2;
    if crate::ui::gilded() {
        crate::ui::panel(p, w, h, ox - 12, oy - 12, SX * scale + 24, SZ * scale + 24);
    }
    for z in 0..SZ {
        for x in 0..SX {
            let y = world.surface_y(x, z);
            render::fill_rect(
                p,
                w,
                h,
                ox + x * scale,
                oy + z * scale,
                scale,
                scale,
                world.get(x, y, z).rgb(0),
            );
        }
    }
    let px = ox + player.x as i32 * scale;
    let pz = oy + player.z as i32 * scale;
    render::fill_rect(p, w, h, px - 3, pz - 3, 7, 7, (255, 90, 70));
    text_center(p, w, h, 16, "WORLD MAP - NORTH IS UP", render::TEXT, 2.0);
    text_center(
        p,
        w,
        h,
        h - 28,
        "M / ESC / CLICK TO CLOSE",
        render::DIM,
        1.5,
    );
}

pub fn run(path: PathBuf, smoke_dir: Option<PathBuf>) -> io::Result<()> {
    run_inner(path, smoke_dir, None, None)
}
pub fn run_map(path: PathBuf, id: &str) -> io::Result<()> {
    let mut game = maps::open(&path, id)?;
    game.configure_input(true, true);
    run_inner(path, None, Some(game), None)
}
pub fn run_pack_test(directory: PathBuf) -> io::Result<()> {
    run_inner(directory.join("world.tcrf"), None, None, Some(directory))
}
fn run_inner(
    path: PathBuf,
    smoke_dir: Option<PathBuf>,
    initial: Option<Game>,
    pack_dir: Option<PathBuf>,
) -> io::Result<()> {
    let mut smoke = smoke_dir.map(Smoke::new).transpose()?;
    let mut pack_smoke = pack_dir.map(PackSmoke::new).transpose()?;
    let testing = smoke.is_some() || pack_smoke.is_some();
    let sdl = sdl2::init().map_err(err)?;
    let video = sdl.video().map_err(err)?;
    sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "0");
    let mut window = video
        .window("Terminal Craft", 1100, 720)
        .position_centered()
        .resizable()
        .allow_highdpi()
        .build()
        .map_err(err)?;
    window.set_minimum_size(640, 420).map_err(err)?;
    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(err)?;
    let creator = canvas.texture_creator();
    let mut texture = None;
    let mut texture_size = (0, 0);
    let mouse = ReleasePointer(sdl.mouse());
    let mut pump = sdl.event_pump().map_err(err)?;
    let event_system = sdl.event().map_err(err)?;
    let mut game: Option<Game> = initial;
    let mut gallery = false;
    let map_worlds = maps::MAPS
        .iter()
        .map(|m| maps::preview(m.id))
        .collect::<io::Result<Vec<_>>>()?;
    let mut selected = if path.exists() { 0 } else { 1 };
    let menu_world = World::generate(42);
    let mut menu_pixels = Vec::new();
    let mut frame = Frame::new(120, 45);
    let mut focused = true;
    let mut last = Instant::now();
    let start = last;
    let mut index = 0;
    let mut fps = 0;
    let mut frames = 0;
    let mut fps_time = last;
    let mut quit = false;
    while !quit {
        let tick = Instant::now();
        let dt = if testing {
            1.0 / 60.0
        } else {
            (tick - last).as_secs_f32().min(0.05)
        };
        last = tick;
        if let Some(s) = smoke.as_mut() {
            s.inject(index, &event_system, canvas.window().id())?;
        }
        if smoke.is_some() {
            if index == 40 {
                canvas.window_mut().set_size(960, 640).map_err(err)?;
            }
            if index == 128 {
                canvas.window_mut().set_size(640, 420).map_err(err)?;
            }
            if index == 44 || index == 131 {
                canvas.window_mut().set_size(1100, 720).map_err(err)?;
            }
        }
        if let Some(s) = pack_smoke.as_mut() {
            if index == 2 {
                canvas.window_mut().set_size(640, 420).map_err(err)?;
            }
            if index == 3 {
                canvas.window_mut().set_size(1100, 720).map_err(err)?;
            }
            s.inject(index, &event_system, canvas.window().id())?;
        }
        let (w, h) = canvas.window().size();
        let menu_count = if gallery { 6 } else { 5 };
        let w = w.max(1);
        let h = h.max(1);
        let mut activate = None;
        for e in pump.poll_iter() {
            // Deterministic QA uses tagged SDL input; real window/focus events still pass.
            // Do not log or consume unrelated physical typing as test commands. Escape aborts QA.
            if testing {
                if matches!(
                    e,
                    SdlEvent::KeyDown {
                        keycode: Some(Keycode::Escape),
                        scancode: Some(_),
                        ..
                    }
                ) {
                    quit = true;
                    continue;
                }
                let scripted = match e {
                    SdlEvent::KeyDown { scancode, .. } | SdlEvent::KeyUp { scancode, .. } => {
                        scancode.is_none()
                    }
                    SdlEvent::MouseMotion { which, .. }
                    | SdlEvent::MouseButtonDown { which, .. }
                    | SdlEvent::MouseButtonUp { which, .. } => which == 0x5443,
                    SdlEvent::MouseWheel { .. } => false,
                    _ => true,
                };
                if !scripted {
                    continue;
                }
            }
            if let Some(s) = smoke.as_mut() {
                s.events.push(format!("{index}: {e:?}"));
            }
            match e {
                SdlEvent::Quit { .. } => quit = true,
                SdlEvent::Window {
                    win_event: WindowEvent::FocusLost,
                    ..
                } => {
                    focused = false;
                    selected = 0;
                    if let Some(g) = game.as_mut() {
                        g.input_event(Event::FocusLost);
                    }
                }
                SdlEvent::Window {
                    win_event: WindowEvent::FocusGained,
                    ..
                } => {
                    focused = true;
                    if let Some(g) = game.as_mut() {
                        g.input_event(Event::FocusGained);
                    }
                }
                SdlEvent::KeyDown {
                    keycode: Some(Keycode::F6),
                    repeat: false,
                    ..
                } => {
                    crate::ui::toggle();
                }
                SdlEvent::KeyDown {
                    keycode: Some(Keycode::F11),
                    repeat: false,
                    ..
                } => {
                    let kind = if canvas.window().fullscreen_state() == FullscreenType::Off {
                        FullscreenType::Desktop
                    } else {
                        FullscreenType::Off
                    };
                    canvas.window_mut().set_fullscreen(kind).map_err(err)?;
                }
                SdlEvent::KeyDown {
                    keycode: Some(k),
                    keymod,
                    repeat,
                    ..
                }
                | SdlEvent::KeyUp {
                    keycode: Some(k),
                    keymod,
                    repeat,
                    ..
                } => {
                    let down = matches!(e, SdlEvent::KeyDown { .. });
                    if let Some(input) = key_event(k, keymod, down, repeat) {
                        let Event::Key(ke) = input else {
                            unreachable!()
                        };
                        if let Some(g) = game.as_mut() {
                            if g.pause_menu()
                                && down
                                && !repeat
                                && matches!(
                                    ke.code,
                                    KeyCode::Up
                                        | KeyCode::Down
                                        | KeyCode::Enter
                                        | KeyCode::Char('w' | 's')
                                )
                            {
                                match ke.code {
                                    KeyCode::Up | KeyCode::Char('w') => {
                                        selected = (selected + 2) % 3
                                    }
                                    KeyCode::Down | KeyCode::Char('s') => {
                                        selected = (selected + 1) % 3
                                    }
                                    KeyCode::Enter => activate = Some(selected),
                                    _ => {}
                                }
                            } else {
                                let was = g.pause_menu();
                                quit |= g.input_event(input);
                                if !was && g.pause_menu() {
                                    selected = 0;
                                }
                            }
                        } else if down && !repeat {
                            match ke.code {
                                KeyCode::Esc | KeyCode::Char('q') => {
                                    if gallery {
                                        gallery = false;
                                        selected = 3;
                                    } else {
                                        quit = true;
                                    }
                                }
                                KeyCode::Up | KeyCode::Char('w') => {
                                    selected = (selected + menu_count - 1) % menu_count
                                }
                                KeyCode::Down | KeyCode::Char('s') => {
                                    selected = (selected + 1) % menu_count
                                }
                                KeyCode::Enter => activate = Some(selected),
                                KeyCode::Char('1'..='6') => {
                                    if let KeyCode::Char(c) = ke.code {
                                        activate = Some(c as usize - '1' as usize);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                SdlEvent::MouseMotion {
                    x, y, xrel, yrel, ..
                } => {
                    if let Some(g) = game.as_mut() {
                        if !g.is_paused() {
                            g.relative_look(xrel as f32, yrel as f32);
                        } else if g.pause_menu() {
                            if let Some(i) = hit_button(&buttons(w as i32, h as i32, 3), x, y) {
                                selected = i;
                            }
                        }
                    } else if let Some(i) =
                        hit_button(&buttons(w as i32, h as i32, menu_count), x, y)
                    {
                        selected = i;
                    }
                }
                SdlEvent::MouseButtonDown {
                    mouse_btn, x, y, ..
                }
                | SdlEvent::MouseButtonUp {
                    mouse_btn, x, y, ..
                } => {
                    let down = matches!(e, SdlEvent::MouseButtonDown { .. });
                    if let Some(g) = game.as_mut() {
                        if g.pause_menu() {
                            if down && mouse_btn == SdlButton::Left {
                                activate = hit_button(&buttons(w as i32, h as i32, 3), x, y);
                            }
                        } else if g.is_paused() {
                            if down && mouse_btn == SdlButton::Left && focused {
                                g.input_event(key(KeyCode::Esc));
                            }
                        } else if let Some(b) = match mouse_btn {
                            SdlButton::Left => Some(MouseButton::Left),
                            SdlButton::Right => Some(MouseButton::Right),
                            SdlButton::Middle => Some(MouseButton::Middle),
                            _ => None,
                        } {
                            g.input_event(Event::Mouse(MouseEvent {
                                kind: if down {
                                    MouseEventKind::Down(b)
                                } else {
                                    MouseEventKind::Up(b)
                                },
                                column: 0,
                                row: 0,
                                modifiers: KeyModifiers::NONE,
                            }));
                        }
                    } else if down && mouse_btn == SdlButton::Left {
                        activate = hit_button(&buttons(w as i32, h as i32, menu_count), x, y);
                    }
                }
                SdlEvent::MouseWheel { y, .. } => {
                    if let Some(g) = game.as_mut() {
                        if y != 0 {
                            g.input_event(Event::Mouse(MouseEvent {
                                kind: if y > 0 {
                                    MouseEventKind::ScrollUp
                                } else {
                                    MouseEventKind::ScrollDown
                                },
                                column: 0,
                                row: 0,
                                modifiers: KeyModifiers::NONE,
                            }));
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some(a) = activate {
            if let Some(g) = game.as_mut() {
                match a {
                    0 => {
                        g.input_event(key(KeyCode::Esc));
                    }
                    1 => {
                        g.save()?;
                        game = None;
                        selected = 0;
                    }
                    2 => quit = true,
                    _ => {}
                }
            } else {
                if gallery {
                    if let Some(map) = maps::MAPS.get(a) {
                        game = Some(maps::open(&path, map.id)?);
                    }
                    gallery = false;
                    selected = 3;
                } else {
                    match a {
                        0 if path.exists() => {
                            game = Some(Game::load(&path)?);
                        }
                        1 | 2 => {
                            let seed = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u32;
                            game = Some(Game::new_map(seed, a == 2, path.clone()));
                        }
                        3 => {
                            gallery = true;
                            selected = 0;
                        }
                        4 => quit = true,
                        _ => {}
                    }
                }
                if let Some(g) = game.as_mut() {
                    g.configure_input(true, true);
                    if !focused {
                        g.input_event(Event::FocusLost);
                    }
                    selected = 0;
                }
            }
        }
        let capture = capture_allowed(
            game.is_some(),
            game.as_ref().is_none_or(Game::is_paused),
            focused,
        ) && !quit;
        if mouse.0.relative_mouse_mode() != capture {
            mouse.0.set_relative_mouse_mode(capture);
        }
        if capture && !mouse.0.relative_mouse_mode() {
            return Err(err(format!(
                "Native pointer lock failed: {}",
                sdl2::get_error()
            )));
        }
        if quit {
            if let Some(g) = game.as_ref() {
                g.save()?;
            }
            break;
        }
        if let Some(g) = game.as_mut() {
            g.update(dt);
        }
        let rgba = if let Some(g) = game.as_ref() {
            let p = g.render_native(&mut frame, w, h, fps);
            if g.pause_menu() {
                panel(
                    p,
                    w as i32,
                    h as i32,
                    "PAUSED",
                    &["RESUME", "SAVE AND MAIN MENU", "SAVE AND QUIT"],
                    selected,
                    true,
                );
            }
            p
        } else {
            menu_pixels.resize((w * h * 4) as usize, 0);
            render::fill_view(
                &mut menu_pixels,
                w as i32,
                h as i32,
                if gallery {
                    &map_worlds[selected.min(4)]
                } else {
                    &menu_world
                },
                if gallery { 4.5 } else { 48.5 },
                if gallery { 32.0 } else { 29.0 },
                48.5,
                if gallery {
                    0.0
                } else {
                    start.elapsed().as_secs_f32() * 0.03
                },
                if gallery { -0.4 } else { -0.30 },
                false,
                0.0,
            );
            panel(
                &mut menu_pixels,
                w as i32,
                h as i32,
                if gallery {
                    "PREBUILT WORLDS"
                } else {
                    "TERMINAL CRAFT"
                },
                &if gallery {
                    maps::MAPS
                        .iter()
                        .map(|m| m.title)
                        .chain(std::iter::once("BACK"))
                        .collect::<Vec<_>>()
                } else {
                    vec![
                        "CONTINUE",
                        "NEW SURVIVAL",
                        "NEW CREATIVE",
                        "PREBUILT WORLDS",
                        "QUIT",
                    ]
                },
                selected,
                path.exists(),
            );
            &mut menu_pixels
        };
        if texture_size != (w, h) {
            texture = Some(
                creator
                    .create_texture_streaming(PixelFormatEnum::RGBA32, w, h)
                    .map_err(err)?,
            );
            texture_size = (w, h);
        }
        let t = texture.as_mut().unwrap();
        t.update(None, rgba, w as usize * 4).map_err(err)?;
        canvas.clear();
        canvas.copy(t, None, None).map_err(err)?;
        canvas.present();
        if let Some(s) = smoke.as_mut() {
            if index == 40 {
                s.resize_ok = (w, h) == (960, 640);
            }
            if index == 129 {
                s.resize_ok &= (w, h) == (640, 420);
            }
            if index == 52 {
                s.fullscreen_ok = canvas.window().fullscreen_state() != FullscreenType::Off;
            }
            if index == 54 {
                s.fullscreen_ok &= canvas.window().fullscreen_state() == FullscreenType::Off;
            }
            s.observe(
                index,
                game.as_ref(),
                mouse.0.relative_mouse_mode(),
                rgba,
                w,
                h,
                tick.elapsed(),
            )?;
        }
        if let Some(s) = pack_smoke.as_mut() {
            s.observe(
                index,
                game.as_ref(),
                &path,
                rgba,
                w,
                h,
                mouse.0.relative_mouse_mode(),
            )?;
        }
        frames += 1;
        if fps_time.elapsed() >= Duration::from_secs(1) {
            fps = frames;
            frames = 0;
            fps_time = Instant::now();
        }
        index += 1;
        if smoke.is_some() && index > 160 {
            return Err(err("Native smoke test did not exit through save-and-quit"));
        }
        let budget = Duration::from_nanos(16_666_667);
        if tick.elapsed() < budget {
            std::thread::sleep(budget - tick.elapsed());
        }
    }
    mouse.0.set_relative_mouse_mode(false);
    mouse.0.show_cursor(true);
    if let Some(s) = pack_smoke {
        s.finish(!mouse.0.relative_mouse_mode())?;
    }
    if let Some(s) = smoke {
        s.finish(&path, !mouse.0.relative_mouse_mode(), index)?;
    }
    Ok(())
}

struct Smoke {
    dir: PathBuf,
    poses: std::collections::BTreeMap<u32, [f32; 5]>,
    captures: std::collections::BTreeMap<u32, bool>,
    themes: std::collections::BTreeMap<u32, bool>,
    menu_returned: bool,
    resize_ok: bool,
    fullscreen_ok: bool,
    times: Vec<f64>,
    events: Vec<String>,
}
impl Smoke {
    fn new(dir: PathBuf) -> io::Result<Self> {
        if dir.exists() {
            return Err(err(
                "Smoke-test output directory must not already exist; normal saves are never used",
            ));
        }
        std::fs::create_dir_all(&dir)?;
        let mut world = World::generate(42);
        for x in 28..69 {
            for z in 28..69 {
                for y in 15..SY {
                    world.set(
                        x,
                        y,
                        z,
                        if y == 15 {
                            crate::block::Block::Slate
                        } else {
                            crate::block::Block::Air
                        },
                    );
                }
            }
        }
        world.save(
            &dir.join("world.tcrf"),
            &[100; 5],
            false,
            0,
            Some(&crate::world::SaveState {
                pose: [48.5, 16.001, 48.5, 0.0, 0.0],
                tool: 0,
                flying: false,
            }),
        )?;
        Ok(Self {
            dir,
            poses: Default::default(),
            captures: Default::default(),
            themes: Default::default(),
            menu_returned: false,
            resize_ok: false,
            fullscreen_ok: false,
            times: Vec::new(),
            events: Vec::new(),
        })
    }
    fn inject(&self, n: u32, events: &sdl2::EventSubsystem, id: u32) -> io::Result<()> {
        let push = |e| events.push_event(e).map_err(err);
        let send = |k, down, mods| {
            push(if down {
                SdlEvent::KeyDown {
                    timestamp: 0,
                    window_id: id,
                    keycode: Some(k),
                    scancode: None,
                    keymod: mods,
                    repeat: false,
                }
            } else {
                SdlEvent::KeyUp {
                    timestamp: 0,
                    window_id: id,
                    keycode: Some(k),
                    scancode: None,
                    keymod: mods,
                    repeat: false,
                }
            })
        };
        let motion = |xrel, yrel| {
            push(SdlEvent::MouseMotion {
                timestamp: 0,
                window_id: id,
                which: 0x5443,
                mousestate: sdl2::mouse::MouseState::from_sdl_state(0),
                x: 550,
                y: 360,
                xrel,
                yrel,
            })
        };
        let click = |x, y| -> io::Result<()> {
            push(SdlEvent::MouseButtonDown {
                timestamp: 0,
                window_id: id,
                which: 0x5443,
                mouse_btn: SdlButton::Left,
                clicks: 1,
                x,
                y,
            })?;
            push(SdlEvent::MouseButtonUp {
                timestamp: 0,
                window_id: id,
                which: 0x5443,
                mouse_btn: SdlButton::Left,
                clicks: 1,
                x,
                y,
            })
        };
        match n {
            6 => {
                let r = buttons(1100, 720, 5)[0];
                click(r.x + r.w / 2, r.y + r.h / 2)?;
            }
            65 => {
                let r = buttons(1100, 720, 3)[0];
                click(r.x + r.w / 2, r.y + r.h / 2)?;
            }
            130 => click(550, 360)?,
            138 => {
                send(Keycode::Return, true, Mod::NOMOD)?;
                send(Keycode::Return, false, Mod::NOMOD)?;
            }
            10 => {
                send(Keycode::W, true, Mod::NOMOD)?;
                send(Keycode::D, true, Mod::NOMOD)?;
            }
            18 => motion(4000, -900)?,
            22 => {
                send(Keycode::W, false, Mod::NOMOD)?;
                send(Keycode::D, false, Mod::NOMOD)?;
            }
            34 => motion(0, 2000)?,
            36 => motion(0, -650)?,
            38 | 72 => send(Keycode::Space, true, Mod::NOMOD)?,
            39 | 88 => send(Keycode::Space, false, Mod::NOMOD)?,
            52 | 54 => send(Keycode::F11, true, Mod::NOMOD)?,
            50 | 120 | 132 => {
                send(Keycode::Escape, true, Mod::NOMOD)?;
                send(Keycode::Escape, false, Mod::NOMOD)?;
            }
            55 => {
                send(Keycode::W, true, Mod::NOMOD)?;
                motion(3000, 1000)?;
            }
            60 => push(SdlEvent::Window {
                timestamp: 0,
                window_id: id,
                win_event: WindowEvent::FocusLost,
            })?,
            62 => push(SdlEvent::Window {
                timestamp: 0,
                window_id: id,
                win_event: WindowEvent::FocusGained,
            })?,
            68 => send(Keycode::W, false, Mod::NOMOD)?,
            70 => send(Keycode::C, true, Mod::NOMOD)?,
            92 => send(Keycode::LShift, true, Mod::LSHIFTMOD)?,
            108 => send(Keycode::LShift, false, Mod::NOMOD)?,
            110 => send(Keycode::G, true, Mod::NOMOD)?,
            114 => send(Keycode::M, true, Mod::NOMOD)?,
            122 | 123 => {
                send(Keycode::F6, true, Mod::NOMOD)?;
                send(Keycode::F6, false, Mod::NOMOD)?;
            }
            124 => send(Keycode::H, true, Mod::NOMOD)?,
            134 => {
                send(Keycode::Down, true, Mod::NOMOD)?;
                send(Keycode::Return, true, Mod::NOMOD)?;
            }
            142 => send(Keycode::Q, true, Mod::LCTRLMOD)?,
            _ => {}
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)] // Explicit image dimensions in the native QA observer.
    fn observe(
        &mut self,
        n: u32,
        game: Option<&Game>,
        captured: bool,
        rgba: &[u8],
        w: u32,
        h: u32,
        time: Duration,
    ) -> io::Result<()> {
        if let Some(g) = game {
            self.poses.insert(n, g.snapshot());
        } else if n >= 134 {
            self.menu_returned = true;
        }
        self.captures.insert(n, captured);
        self.themes.insert(n, crate::ui::gilded());
        self.times.push(time.as_secs_f64() * 1000.0);
        let name = match n {
            3 => Some("menu"),
            30 => Some("gameplay"),
            56 => Some("pause"),
            116 => Some("map"),
            126 => Some("controls"),
            129 => Some("controls-compact"),
            _ => None,
        };
        if let Some(name) = name {
            use std::io::Write;
            let mut f = std::fs::File::create(self.dir.join(format!("{name}.ppm")))?;
            write!(f, "P6\n{w} {h}\n255\n")?;
            let rgb: Vec<u8> = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| p[..3].iter().copied())
                .collect();
            f.write_all(&rgb)?;
        }
        Ok(())
    }
    fn finish(mut self, path: &Path, released: bool, frames: u32) -> io::Result<()> {
        std::fs::write(self.dir.join("events.log"), self.events.join("\n"))?;
        let states = self
            .poses
            .iter()
            .map(|(n, p)| format!("{{\"frame\":{n},\"pose\":{p:?}}}"))
            .collect::<Vec<_>>()
            .join(",");
        std::fs::write(self.dir.join("poses.json"), format!("[{states}]\n"))?;
        let pose = |n| self.poses.get(&n).copied().unwrap_or([f32::NAN; 5]);
        let a = pose(9);
        let b = pose(22);
        let s = pose(25);
        let t = pose(31);
        let paused = pose(50);
        let still = pose(63);
        let walk = ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        let stopped = (s[0] - t[0]).abs() + (s[2] - t[2]).abs() < 0.0001;
        let frozen = paused
            .iter()
            .zip(still)
            .all(|(a, b)| (*a - b).abs() < 0.0001);
        let turn = (pose(19)[3] - a[3]).abs();
        let pitch = self
            .poses
            .values()
            .map(|p| p[4].abs())
            .fold(0.0_f32, f32::max);
        let jump = pose(45)[1] > pose(37)[1] + 0.4;
        let ascend = pose(87)[1] > pose(71)[1] + 1.0;
        let descend = pose(108)[1] < pose(91)[1] - 1.0;
        let locked = self.captures.get(&20) == Some(&true);
        let unlocked =
            self.captures.get(&55) == Some(&false) && self.captures.get(&62) == Some(&false);
        let saved = World::load(path)?
            .4
            .ok_or_else(|| err("No saved player state"))?
            .pose;
        let saved_exact = saved == pose(141);
        let theme_roundtrip = self.themes.get(&121) == self.themes.get(&123)
            && self.themes.get(&121) != self.themes.get(&122);
        let passed = self.resize_ok
            && self.fullscreen_ok
            && frames == 142
            && walk > 0.1
            && stopped
            && frozen
            && turn > std::f32::consts::TAU
            && pitch > 1.55
            && jump
            && ascend
            && descend
            && locked
            && unlocked
            && released
            && self.menu_returned
            && saved_exact
            && theme_roundtrip;
        self.times.sort_by(f64::total_cmp);
        let median = self.times[self.times.len() / 2];
        let report=format!("{{\n  \"passed\":{passed},\n  \"frames\":{frames},\n  \"walk_distance\":{walk},\n  \"immediate_stop\":{stopped},\n  \"pause_and_focus_freeze\":{frozen},\n  \"turn_radians\":{turn},\n  \"max_pitch\":{pitch},\n  \"jump\":{jump},\n  \"flight_ascend\":{ascend},\n  \"flight_descend\":{descend},\n  \"pointer_locked\":{locked},\n  \"pointer_released_on_pause\":{unlocked},\n  \"pointer_released_on_exit\":{released},\n  \"main_menu_return\":{},\n  \"save_reload_exact\":{saved_exact},\n  \"native_frame_median_ms\":{median}\n}}\n",self.menu_returned);
        let report = report.replacen(
            "{\n",
            &format!(
                "{{\n  \"resize\":{},\n  \"fullscreen_round_trip\":{},\n",
                self.resize_ok, self.fullscreen_ok
            ),
            1,
        );
        let report = report.replacen(
            "{\n",
            &format!(
                "{{\n  \"ui_theme\":\"{}\",\n  \"ui_theme_round_trip\":{theme_roundtrip},\n",
                crate::ui::name()
            ),
            1,
        );
        std::fs::write(self.dir.join("report.json"), &report)?;
        println!("{report}");
        if passed {
            Ok(())
        } else {
            Err(err("Native smoke test failed; inspect report.json"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_keys_preserve_press_repeat_release_and_modifiers() {
        let Event::Key(k) = key_event(Keycode::W, Mod::LCTRLMOD, true, false).unwrap() else {
            panic!()
        };
        assert_eq!(k.code, KeyCode::Char('w'));
        assert_eq!(k.kind, KeyEventKind::Press);
        assert!(k.modifiers.contains(KeyModifiers::CONTROL));
        let Event::Key(k) = key_event(Keycode::W, Mod::NOMOD, false, false).unwrap() else {
            panic!()
        };
        assert_eq!(k.kind, KeyEventKind::Release);
        let Event::Key(k) = key_event(Keycode::Space, Mod::NOMOD, true, true).unwrap() else {
            panic!()
        };
        assert_eq!(k.kind, KeyEventKind::Repeat);
    }
    #[test]
    fn native_capture_is_only_allowed_during_focused_gameplay() {
        for game in [false, true] {
            for paused in [false, true] {
                for focused in [false, true] {
                    assert_eq!(
                        capture_allowed(game, paused, focused),
                        game && !paused && focused
                    );
                }
            }
        }
    }
    #[test]
    fn native_buttons_remain_clickable_at_all_window_sizes() {
        for (w, h) in [(640, 420), (1100, 720), (1800, 1100)] {
            for count in [3, 5, 6] {
                let r = buttons(w, h, count);
                for (i, b) in r.iter().enumerate() {
                    assert!(b.x >= 0 && b.y >= 0 && b.x + b.w <= w && b.y + b.h <= h);
                    assert_eq!(hit_button(&r, b.x + b.w / 2, b.y + b.h / 2), Some(i));
                }
            }
        }
    }
}
