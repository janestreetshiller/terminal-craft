use crate::block::{HOTBAR, Inventory};
use crate::player::{raycast, Player};
use crate::render::Frame;
use crate::world::World;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode, MouseButton, MouseEvent, MouseEventKind};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::queue;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct Game {
    world: World,
    player: Player,
    inv: Inventory,
    slot: usize,
    creative: bool,
    flying: bool,
    map: bool,
    debug: bool,
    toast: String,
    toast_until: Instant,
    last_space: Instant,
    save_path: PathBuf,
    keys: Keys,
    mouse: (i32, i32),
    mouse_armed: bool,
    lmb: bool,
    drag: u32,
    pixels: bool,
    term_cols: u16,
    term_rows: u16,
    cell_w: f32,
    cell_h: f32,
    pixel_mouse: bool,
}

struct Hold {
    down: bool,
    at: Instant,
}

impl Hold {
    fn new() -> Self {
        Self {
            down: false,
            at: Instant::now() - Duration::from_secs(8),
        }
    }
    fn set(&mut self, down: bool) {
        self.down = down;
        if down {
            self.at = Instant::now();
        }
    }
    fn held(&self, saw_release: bool) -> bool {
        if !self.down {
            return false;
        }
        saw_release || self.at.elapsed() < Duration::from_millis(400)
    }
}

struct Keys {
    w: Hold,
    a: Hold,
    s: Hold,
    d: Hold,
    space: Hold,
    sneak: Hold,
    left: Hold,
    right: Hold,
    up: Hold,
    down: Hold,
    jump: bool,
    saw_release: bool,
    shift: bool,
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            w: Hold::new(),
            a: Hold::new(),
            s: Hold::new(),
            d: Hold::new(),
            space: Hold::new(),
            sneak: Hold::new(),
            left: Hold::new(),
            right: Hold::new(),
            up: Hold::new(),
            down: Hold::new(),
            jump: false,
            saw_release: false,
            shift: false,
        }
    }
}

impl Game {
    pub fn new_map(seed: u32, creative: bool, save_path: PathBuf) -> Self {
        let world = World::generate(seed);
        let player = Player::spawn(&world);
        Self {
            world,
            player,
            inv: if creative {
                Inventory {
                    terra: 64,
                    slate: 64,
                    wood: 64,
                    sand: 64,
                    gem: 16,
                }
            } else {
                Inventory::default()
            },
            slot: 0,
            creative,
            flying: creative,
            map: false,
            debug: false,
            toast: String::new(),
            toast_until: Instant::now(),
            last_space: Instant::now() - Duration::from_secs(1),
            save_path,
            keys: Keys::default(),
            mouse: (0, 0),
            mouse_armed: false,
            lmb: false,
            drag: 0,
            pixels: crate::kitty_gfx::available(),
            term_cols: 120,
            term_rows: 40,
            cell_w: 8.0,
            cell_h: 16.0,
            pixel_mouse: false,
        }
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let (world, inv, creative, slot) = World::load(path)?;
        let player = Player::spawn(&world);
        Ok(Self {
            world,
            player,
            inv: Inventory::unpack(inv),
            slot: (slot as usize).min(5),
            creative,
            flying: creative,
            map: false,
            debug: false,
            toast: "CONTINUE".into(),
            toast_until: Instant::now() + Duration::from_secs(2),
            last_space: Instant::now() - Duration::from_secs(1),
            save_path: path.to_path_buf(),
            keys: Keys::default(),
            mouse: (0, 0),
            mouse_armed: false,
            lmb: false,
            drag: 0,
            pixels: crate::kitty_gfx::available(),
            term_cols: 120,
            term_rows: 40,
            cell_w: 8.0,
            cell_h: 16.0,
            pixel_mouse: false,
        })
    }

    fn save(&self) {
        if let Err(e) = self.world.save(
            &self.save_path,
            &self.inv.pack(),
            self.creative,
            self.slot as u8,
        ) {
            // keep playing; toast on next frame via eprintln is wrong in alt screen
            let _ = e;
        }
    }

    fn toast(&mut self, s: &str) {
        self.toast = s.to_string();
        self.toast_until = Instant::now() + Duration::from_millis(1400);
    }

    pub fn run(&mut self, out: &mut impl Write) -> io::Result<()> {
        let (mut cols, mut rows) = terminal::size()?;
        let mut frame = Frame::new(cols, rows);
        let mut metrics = stdout_metrics(cols, rows);
        let mut last = Instant::now();
        let mut fps = 0u32;
        let mut frames = 0u32;
        let mut fps_t = Instant::now();

        loop {
            let now = Instant::now();
            let dt = (now - last).as_secs_f32().min(0.05);
            last = now;

            while event::poll(Duration::from_millis(0))? {
                match event::read()? {
                    Event::Key(k) if k.kind == KeyEventKind::Release => {
                        self.keys.saw_release = true;
                        if !k.modifiers.contains(KeyModifiers::SHIFT) {
                            self.keys.shift = false;
                        }
                        self.set_hold(k.code, false);
                    }
                    Event::Key(k) if k.kind == KeyEventKind::Repeat => {
                        self.keys.shift = k.modifiers.contains(KeyModifiers::SHIFT);
                        self.set_hold(k.code, true);
                    }
                    Event::Key(k) => {
                        self.keys.shift = k.modifiers.contains(KeyModifiers::SHIFT);
                        if self.handle_key(k) {
                            self.save();
                            return Ok(());
                        }
                    }
                    Event::Mouse(m) => self.handle_mouse(m),
                    Event::Resize(c, r) => {
                        cols = c;
                        rows = r;
                        frame.resize(c, r);
                        metrics = stdout_metrics(c, r);
                        queue!(out, Clear(ClearType::All))?;
                    }
                    _ => {}
                }
            }

            let rel = self.keys.saw_release;
            let mx = (self.keys.d.held(rel) as i32 - self.keys.a.held(rel) as i32) as f32;
            let mz = (self.keys.w.held(rel) as i32 - self.keys.s.held(rel) as i32) as f32;
            let jump = self.keys.jump;
            if jump && (self.player.on_ground || self.flying) {
                self.keys.jump = false;
            }
            self.player.flying = self.flying;
            self.player.tick(
                &self.world,
                mx,
                mz,
                jump,
                self.keys.sneak.held(rel) || self.keys.shift,
                self.keys.space.held(rel),
                dt,
                self.creative && self.flying,
            );
            self.term_cols = cols;
            self.term_rows = rows;
            self.cell_w = metrics.cell_w.max(1) as f32;
            self.cell_h = metrics.cell_h.max(1) as f32;
            let look_s = 2.8 * dt;
            if self.keys.left.held(rel) {
                self.player.look(-look_s, 0.0);
            }
            if self.keys.right.held(rel) {
                self.player.look(look_s, 0.0);
            }
            if self.keys.up.held(rel) {
                self.player.look(0.0, look_s);
            }
            if self.keys.down.held(rel) {
                self.player.look(0.0, -look_s);
            }

            frames += 1;
            if fps_t.elapsed() >= Duration::from_secs(1) {
                fps = frames;
                frames = 0;
                fps_t = Instant::now();
            }

            let toast = if now < self.toast_until {
                self.toast.as_str()
            } else {
                ""
            };
            let deny = toast == "NEED RES";
            frame.resize(cols, rows);
            frame.draw(
                &self.world,
                &self.player,
                &self.inv,
                self.slot,
                self.creative,
                toast,
                fps,
                self.debug,
                self.map,
                self.pixels,
                deny,
                metrics,
            );
            out.write_all(frame.as_bytes())?;
            out.flush()?;
        }
    }

    fn handle_key(&mut self, k: KeyEvent) -> bool {
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            return true;
        }
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                return true;
            }
            KeyCode::Esc => return true,
            KeyCode::Char('w')
            | KeyCode::Char('W')
            | KeyCode::Char('a')
            | KeyCode::Char('A')
            | KeyCode::Char('s')
            | KeyCode::Char('S')
            | KeyCode::Char('d')
            | KeyCode::Char('D')
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::Char('z')
            | KeyCode::Char('Z')
            | KeyCode::Modifier(_) => self.set_hold(k.code, true),
            KeyCode::Char(' ') => {
                let now = Instant::now();
                if self.creative && now.duration_since(self.last_space) < Duration::from_millis(280) {
                    self.flying = !self.flying;
                    self.toast(if self.flying { "FLY" } else { "WALK" });
                }
                self.last_space = now;
                self.keys.space.set(true);
                self.keys.jump = true;
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.creative = !self.creative;
                if self.creative {
                    self.flying = true;
                    self.toast("CREATIVE");
                } else {
                    self.flying = false;
                    self.toast("SURVIVAL");
                }
            }
            KeyCode::Char('m') | KeyCode::Char('M') => self.map = !self.map,
            KeyCode::F(3) => self.debug = !self.debug,
            KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Char('f') | KeyCode::Char('F') => {
                self.break_block();
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Tab => {
                self.place_block();
            }
            KeyCode::Char('r') | KeyCode::Char('R') => self.save_toast(),
            KeyCode::Char(d) if d.is_ascii_digit() => {
                let n = d.to_digit(10).unwrap_or(0) as usize;
                if (1..=6).contains(&n) {
                    self.slot = n - 1;
                }
            }
            KeyCode::Char('[') => self.slot = (self.slot + 5) % 6,
            KeyCode::Char(']') => self.slot = (self.slot + 1) % 6,
            _ => {}
        }
        false
    }

    fn set_hold(&mut self, code: KeyCode, down: bool) {
        match code {
            KeyCode::Char('w') | KeyCode::Char('W') => self.keys.w.set(down),
            KeyCode::Char('a') | KeyCode::Char('A') => self.keys.a.set(down),
            KeyCode::Char('s') | KeyCode::Char('S') => self.keys.s.set(down),
            KeyCode::Char('d') | KeyCode::Char('D') => self.keys.d.set(down),
            KeyCode::Left => self.keys.left.set(down),
            KeyCode::Right => self.keys.right.set(down),
            KeyCode::Up => self.keys.up.set(down),
            KeyCode::Down => self.keys.down.set(down),
            KeyCode::Char(' ') => self.keys.space.set(down),
            KeyCode::Char('z') | KeyCode::Char('Z') => self.keys.sneak.set(down),
            KeyCode::Modifier(m)
                if matches!(m, ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift) =>
            {
                self.keys.shift = down;
            }
            _ => {}
        }
    }

    fn handle_mouse(&mut self, m: MouseEvent) {
        let x = m.column as i32;
        let y = m.row as i32;
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.lmb = true;
                self.drag = 0;
                self.mouse = (x, y);
                self.mouse_armed = true;
            }
            MouseEventKind::Down(MouseButton::Right) => self.place_block(),
            MouseEventKind::Up(MouseButton::Left) => {
                self.lmb = false;
                if self.drag < 4 {
                    self.break_block();
                }
            }
            MouseEventKind::Drag(_) | MouseEventKind::Moved => {
                if x >= self.term_cols as i32 || y >= self.term_rows as i32 {
                    self.pixel_mouse = true;
                }
                if !self.mouse_armed {
                    self.mouse = (x, y);
                    self.mouse_armed = true;
                    return;
                }
                let (lx, ly) = self.mouse;
                let dx = x - lx;
                let dy = y - ly;
                self.mouse = (x, y);
                if dx == 0 && dy == 0 {
                    return;
                }
                if self.lmb {
                    self.drag = self.drag.saturating_add(dx.unsigned_abs() + dy.unsigned_abs());
                }
                // 1016 reports pixels. Cell reports are scaled to pixels first.
                let sens = std::env::var("TUICRAFT_SENS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0024f32);
                let (px, py) = if self.pixel_mouse {
                    (dx as f32, dy as f32)
                } else {
                    (dx as f32 * self.cell_w, dy as f32 * self.cell_h)
                };
                self.player.look(-px * sens, -py * sens);
            }
            MouseEventKind::ScrollUp => self.slot = (self.slot + 5) % 6,
            MouseEventKind::ScrollDown => self.slot = (self.slot + 1) % 6,
            _ => {}
        }
    }

    fn break_block(&mut self) {
        let (ox, oy, oz) = self.player.eye();
        let (dx, dy, dz) = self.player.look_dir();
        if let Some(hit) = raycast(&self.world, ox, oy, oz, dx, dy, dz, 6.0) {
            if let Some(res) = hit.block.harvest() {
                self.inv.add(res, 1);
                self.toast(res.name());
            } else {
                self.toast(hit.block.name());
            }
            self.world.set(hit.x, hit.y, hit.z, crate::block::Block::Air);
        }
    }

    fn place_block(&mut self) {
        let (ox, oy, oz) = self.player.eye();
        let (dx, dy, dz) = self.player.look_dir();
        if let Some(hit) = raycast(&self.world, ox, oy, oz, dx, dy, dz, 6.0) {
            let b = HOTBAR[self.slot];
            if !self.inv.try_pay(b, self.creative) {
                self.toast("NEED RES");
                return;
            }
            // don't place inside player
            let px = hit.px;
            let py = hit.py;
            let pz = hit.pz;
            if (px as f32 - self.player.x).abs() < 0.8
                && (pz as f32 - self.player.z).abs() < 0.8
                && py as f32 >= self.player.y
                && (py as f32) < self.player.y + 1.8
            {
                self.toast("BLOCKED");
                return;
            }
            self.world.set(px, py, pz, b);
            self.toast(b.name());
        }
    }

    fn save_toast(&mut self) {
        self.save();
        self.toast("SAVED");
    }
}

fn stdout_metrics(cols: u16, rows: u16) -> crate::kitty::Metrics {
    use std::os::fd::AsRawFd;
    crate::kitty::Metrics::from_fd(io::stdout().as_raw_fd()).unwrap_or(crate::kitty::Metrics {
        cols,
        rows,
        cell_w: 8,
        cell_h: 16,
        win_w: cols as u32 * 8,
        win_h: rows as u32 * 16,
    })
}
