use crate::block::{Inventory, HOTBAR};
use crate::player::{raycast, Player};
use crate::render::Frame;
use crate::viewmodel::{Tool, ViewModel};
use crate::world::World;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode, MouseButton,
    MouseEvent, MouseEventKind,
};
use crossterm::queue;
use crossterm::terminal::{self, Clear, ClearType};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn frame_budget(paused: bool) -> Duration {
    if paused {
        Duration::from_millis(50)
    } else {
        Duration::from_nanos(16_666_667)
    }
}

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
    view: ViewModel,
    mine_progress: f32,
    mine_target: Option<(i32, i32, i32)>,
    mine_key: Hold,
    help: bool,
    paused: bool,
    focused: bool,
    enhanced_keys: bool,
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
    sprint: bool,
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
            sprint: false,
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
            view: ViewModel::default(),
            mine_progress: 0.0,
            mine_target: None,
            mine_key: Hold::new(),
            help: false,
            paused: false,
            focused: true,
            enhanced_keys: false,
        }
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let (world, inv, creative, slot, state) = World::load(path)?;
        let mut player = Player::spawn(&world);
        let mut view = ViewModel::default();
        let mut flying = creative;
        if let Some(s) = state {
            [player.x, player.y, player.z, player.yaw, player.pitch] = s.pose;
            flying = creative && s.flying;
            view.tool = match s.tool {
                1 => Tool::Block,
                2 => Tool::Pickaxe,
                3 => Tool::Axe,
                4 => Tool::Shovel,
                _ => Tool::Hand,
            };
        }
        Ok(Self {
            world,
            player,
            inv: Inventory::unpack(inv),
            slot: (slot as usize).min(5),
            creative,
            flying,
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
            view,
            mine_progress: 0.0,
            mine_target: None,
            mine_key: Hold::new(),
            help: false,
            paused: false,
            focused: true,
            enhanced_keys: false,
        })
    }

    pub(crate) fn save(&self) -> io::Result<()> {
        self.world.save(
            &self.save_path,
            &self.inv.pack(),
            self.creative,
            self.slot as u8,
            Some(&crate::world::SaveState {
                pose: [
                    self.player.x,
                    self.player.y,
                    self.player.z,
                    self.player.yaw,
                    self.player.pitch,
                ],
                tool: self.view.tool as u8,
                flying: self.flying,
            }),
        )
    }

    fn toast(&mut self, s: &str) {
        self.toast = s.to_string();
        self.toast_until = Instant::now() + Duration::from_millis(1400);
    }

    fn step_player(&mut self, dt: f32, mx: f32, mz: f32, jump: bool) {
        self.player.flying = self.flying;
        let before = (self.player.x, self.player.z);
        if !self.is_paused() {
            self.player.sprinting = self.keys.sprint;
            self.player.tick(
                &self.world,
                mx,
                mz,
                jump,
                self.keys.sneak.held(self.keys.saw_release) || self.keys.shift,
                self.keys.space.held(self.keys.saw_release),
                dt,
                self.creative && self.flying,
            );
        }
        let distance = (self.player.x - before.0).abs() + (self.player.z - before.1).abs();
        self.view.advance(
            dt,
            distance > 0.0001 && self.player.on_ground && !self.flying,
        );
    }

    pub(crate) fn update(&mut self, dt: f32) {
        if self.is_paused() {
            return;
        }
        let rel = self.keys.saw_release;
        let mx = (self.keys.d.held(rel) as i32 - self.keys.a.held(rel) as i32) as f32;
        let mz = (self.keys.w.held(rel) as i32 - self.keys.s.held(rel) as i32) as f32;
        let jump = self.keys.jump;
        if jump && (self.player.on_ground || self.flying) {
            self.keys.jump = false;
        }
        self.step_player(dt, mx, mz, jump);
        self.tick_mining(dt);
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
    }

    pub(crate) fn snapshot(&self) -> [f32; 5] {
        [
            self.player.x,
            self.player.y,
            self.player.z,
            self.player.yaw,
            self.player.pitch,
        ]
    }
    pub(crate) fn pause_menu(&self) -> bool {
        self.paused && !self.help && !self.map
    }
    pub(crate) fn render_native<'a>(
        &self,
        frame: &'a mut Frame,
        w: u32,
        h: u32,
        fps: u32,
    ) -> &'a mut [u8] {
        let metrics = crate::kitty::Metrics {
            cols: w as u16,
            rows: h as u16,
            cell_w: 1,
            cell_h: 1,
            win_w: w,
            win_h: h,
        };
        let toast = if Instant::now() < self.toast_until {
            self.toast.as_str()
        } else {
            ""
        };
        frame.draw_world_pixels(
            &self.world,
            &self.player,
            &self.inv,
            self.slot,
            self.creative,
            toast,
            fps,
            self.debug,
            toast == "NEED RES",
            metrics,
            &self.view,
            self.mine_progress,
            self.help,
            false,
        );
        if self.map {
            crate::native::paint_map(
                frame.rgba_mut(),
                w as i32,
                h as i32,
                &self.world,
                &self.player,
            );
        }
        frame.rgba_mut()
    }

    pub(crate) fn configure_input(&mut self, enhanced: bool, pixels: bool) {
        self.enhanced_keys = enhanced;
        self.pixel_mouse = pixels;
        self.reset_controls();
    }

    fn reset_controls(&mut self) {
        self.keys = Keys::default();
        self.keys.saw_release = self.enhanced_keys;
        self.mine_key.set(false);
        self.lmb = false;
        self.mouse_armed = false;
        self.mine_progress = 0.0;
        self.mine_target = None;
    }

    pub(crate) fn is_paused(&self) -> bool {
        self.paused || self.help || self.map || !self.focused
    }

    pub(crate) fn relative_look(&mut self, dx: f32, dy: f32) {
        if self.is_paused() {
            return;
        }
        let sens = std::env::var("TERMINAL_CRAFT_SENS")
            .or_else(|_| std::env::var("TUICRAFT_SENS"))
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .filter(|v| v.is_finite() && *v > 0.0)
            .unwrap_or(0.0024);
        self.player.look(
            dx * sens.clamp(0.0001, 0.02),
            -dy * sens.clamp(0.0001, 0.02),
        );
    }

    pub(crate) fn input_event(&mut self, event: Event) -> bool {
        match event {
            Event::FocusLost => {
                self.focused = false;
                self.paused = true;
                self.reset_controls();
            }
            Event::FocusGained => {
                self.focused = true;
                self.mouse_armed = false;
            }
            Event::Key(k) => {
                self.keys.shift = k.modifiers.contains(KeyModifiers::SHIFT);
                self.keys.sprint = k.modifiers.contains(KeyModifiers::CONTROL);
                if k.kind == KeyEventKind::Release {
                    self.keys.saw_release = true;
                    self.set_hold(k.code, false);
                } else if self.focused {
                    if k.kind == KeyEventKind::Repeat {
                        // Enhanced input is held by press/release; a late repeat must not re-arm it.
                        if !self.keys.saw_release && !self.is_paused() {
                            self.set_hold(k.code, true);
                        }
                    } else {
                        return self.handle_key(k);
                    }
                }
            }
            Event::Mouse(m) => self.handle_mouse(m),
            _ => {}
        }
        false
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
                    Event::Resize(c, r) => {
                        cols = c;
                        rows = r;
                        frame.resize(c, r);
                        metrics = stdout_metrics(c, r);
                        queue!(out, Clear(ClearType::All))?;
                    }
                    other => {
                        if self.input_event(other) {
                            self.save()?;
                            return Ok(());
                        }
                    }
                }
            }

            self.term_cols = cols;
            self.term_rows = rows;
            self.cell_w = metrics.cell_w.max(1) as f32;
            self.cell_h = metrics.cell_h.max(1) as f32;
            self.update(dt);
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
                &self.view,
                self.mine_progress,
                self.help || self.paused,
            );
            out.write_all(frame.as_bytes())?;
            out.flush()?;
            // Bound CPU use without tying gameplay speed to frame rate.
            let spent = now.elapsed();
            let budget = frame_budget(self.help || self.map);
            if spent < budget {
                std::thread::sleep(budget - spent);
            }
        }
    }

    fn handle_key(&mut self, k: KeyEvent) -> bool {
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            return true;
        }
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('q') {
            return true;
        }
        if k.code == KeyCode::Esc {
            if self.help || self.map {
                self.help = false;
                self.map = false;
                self.paused = false;
            } else {
                self.paused = !self.paused;
            }
            self.reset_controls();
            return false;
        }
        if self.paused && k.code == KeyCode::Enter {
            self.paused = false;
            self.reset_controls();
            return false;
        }
        if self.paused && matches!(k.code, KeyCode::Char('q' | 'Q')) {
            return true;
        }
        if self.is_paused()
            && !matches!(
                k.code,
                KeyCode::Char('h' | 'H' | 'i' | 'I' | 'm' | 'M' | 'r' | 'R') | KeyCode::F(3)
            )
        {
            return false;
        }
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                return true;
            }
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
                if self.creative && now.duration_since(self.last_space) < Duration::from_millis(280)
                {
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
            KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('i') | KeyCode::Char('I') => {
                self.help = !self.help;
                self.paused = false;
                self.map = false;
                self.reset_controls();
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.map = !self.map;
                self.paused = false;
                self.help = false;
                self.reset_controls();
            }
            KeyCode::F(3) => self.debug = !self.debug,
            KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Char('f') | KeyCode::Char('F') => {
                self.mine_key.set(true);
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Tab => {
                self.place_block();
            }
            KeyCode::Char('g') | KeyCode::Char('G') if self.creative => {
                self.flying = !self.flying;
                self.toast(if self.flying { "FLY" } else { "WALK" });
            }
            KeyCode::Char('r') | KeyCode::Char('R') => self.save_toast(),
            KeyCode::Char(d) if d.is_ascii_digit() => {
                let n = d.to_digit(10).unwrap_or(0) as usize;
                if (1..=6).contains(&n) {
                    self.slot = n - 1;
                    self.view.tool = Tool::Block;
                } else {
                    self.view.tool = match n {
                        7 => Tool::Pickaxe,
                        8 => Tool::Axe,
                        9 => Tool::Shovel,
                        _ => Tool::Hand,
                    };
                }
                self.mine_progress = 0.0;
                self.toast(self.view.tool.name());
            }
            KeyCode::Char('[') => {
                self.slot = (self.slot + 5) % 6;
                self.view.tool = Tool::Block;
            }
            KeyCode::Char(']') => {
                self.slot = (self.slot + 1) % 6;
                self.view.tool = Tool::Block;
            }
            _ => {}
        }
        false
    }

    fn set_hold(&mut self, code: KeyCode, down: bool) {
        match code {
            KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Char('f') | KeyCode::Char('F') => {
                self.mine_key.set(down)
            }
            KeyCode::Char('w') | KeyCode::Char('W') => self.keys.w.set(down),
            KeyCode::Char('a') | KeyCode::Char('A') => self.keys.a.set(down),
            KeyCode::Char('s') | KeyCode::Char('S') => self.keys.s.set(down),
            KeyCode::Char('d') | KeyCode::Char('D') => self.keys.d.set(down),
            KeyCode::Left => self.keys.left.set(down),
            KeyCode::Right => self.keys.right.set(down),
            KeyCode::Up => self.keys.up.set(down),
            KeyCode::Down => self.keys.down.set(down),
            KeyCode::Char(' ') => {
                self.keys.space.set(down);
                if !down {
                    self.keys.jump = false;
                }
            }
            KeyCode::Char('z') | KeyCode::Char('Z') => self.keys.sneak.set(down),
            KeyCode::Modifier(ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift) => {
                self.keys.shift = down;
            }
            KeyCode::Modifier(ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl) => {
                self.keys.sprint = down
            }
            _ => {}
        }
    }

    fn handle_mouse(&mut self, m: MouseEvent) {
        if self.is_paused() {
            self.mouse_armed = false;
            return;
        }
        let x = m.column as i32;
        let y = m.row as i32;
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.view.swing();
                self.lmb = true;
                self.drag = 0;
                self.mouse = (x, y);
                self.mouse_armed = true;
            }
            MouseEventKind::Down(MouseButton::Right) => self.place_block(),
            MouseEventKind::Up(MouseButton::Left) => {
                self.lmb = false;
                self.mine_progress = 0.0;
                self.mine_target = None;
            }
            MouseEventKind::Drag(_) | MouseEventKind::Moved => {
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
                    self.drag = self
                        .drag
                        .saturating_add(dx.unsigned_abs() + dy.unsigned_abs());
                }
                // 1016 reports pixels. Cell reports are scaled to pixels first.
                let (px, py) = if self.pixel_mouse {
                    (dx as f32, dy as f32)
                } else {
                    (dx as f32 * self.cell_w, dy as f32 * self.cell_h)
                };
                self.relative_look(px, py);
            }
            MouseEventKind::ScrollUp => {
                self.slot = (self.slot + 5) % 6;
                self.view.tool = Tool::Block;
            }
            MouseEventKind::ScrollDown => {
                self.slot = (self.slot + 1) % 6;
                self.view.tool = Tool::Block;
            }
            _ => {}
        }
    }

    fn tick_mining(&mut self, dt: f32) {
        if self.is_paused() || !(self.lmb || self.mine_key.held(self.keys.saw_release)) {
            self.mine_target = None;
            self.mine_progress = 0.0;
            return;
        }
        self.view.swing();
        let (ox, oy, oz) = self.player.eye();
        let (dx, dy, dz) = self.player.look_dir();
        if let Some(hit) = raycast(&self.world, ox, oy, oz, dx, dy, dz, 6.0) {
            let target = Some((hit.x, hit.y, hit.z));
            if target != self.mine_target {
                self.mine_progress = 0.0;
                self.mine_target = target;
            }
            let duration = if self.creative {
                0.12
            } else {
                self.view.tool.mine_time(hit.block)
            };
            self.mine_progress += dt / duration;
            if self.mine_progress >= 1.0 {
                self.break_block();
                self.mine_progress = 0.0;
                self.mine_target = None;
            }
        } else {
            self.mine_progress = 0.0;
            self.mine_target = None;
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
            self.world
                .set(hit.x, hit.y, hit.z, crate::block::Block::Air);
        }
    }

    fn place_block(&mut self) {
        if self.is_paused() {
            return;
        }
        self.view.swing();
        let (ox, oy, oz) = self.player.eye();
        let (dx, dy, dz) = self.player.look_dir();
        if let Some(hit) = raycast(&self.world, ox, oy, oz, dx, dy, dz, 6.0) {
            let b = HOTBAR[self.slot];
            // Validate placement before spending resources.
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
            if !(0..crate::world::SX).contains(&px)
                || !(0..crate::world::SY).contains(&py)
                || !(0..crate::world::SZ).contains(&pz)
                || self.world.get(px, py, pz).is_solid()
            {
                self.toast("BLOCKED");
                return;
            }
            if !self.inv.try_pay(b, self.creative) {
                self.toast("NEED RES");
                return;
            }
            self.world.set(px, py, pz, b);
            self.toast(b.name());
        }
    }

    fn save_toast(&mut self) {
        match self.save() {
            Ok(()) => self.toast("SAVED"),
            Err(_) => self.toast("SAVE FAILED"),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Block;
    use crate::viewmodel::Tool;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn fixture() -> Game {
        let mut g = Game::new_map(
            42,
            false,
            std::env::temp_dir().join("terminal-craft-unused.tcrf"),
        );
        g.player.x = 20.5;
        g.player.y = 20.0;
        g.player.z = 20.5;
        g.player.yaw = 0.0;
        g.player.pitch = 0.0;
        g.world.set(21, 21, 20, Block::Slate);
        g.inv.terra = 5;
        g
    }

    #[test]
    fn flight_toggle_repeat_and_space_release_are_explicit() {
        let mut g = fixture();
        g.configure_input(true, true);
        g.input_event(Event::Key(key('c')));
        assert!(g.flying);
        g.input_event(Event::Key(key('g')));
        assert!(!g.flying);
        g.input_event(Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('g'),
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        )));
        assert!(!g.flying);
        g.input_event(Event::Key(key('g')));
        assert!(g.flying);
        g.input_event(Event::Key(key(' ')));
        g.input_event(Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char(' '),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        )));
        assert!(!g.keys.jump);
    }

    #[test]
    fn late_repeat_does_not_restart_a_released_key() {
        let mut g = fixture();
        g.configure_input(true, true);
        for kind in [
            KeyEventKind::Press,
            KeyEventKind::Release,
            KeyEventKind::Repeat,
        ] {
            g.input_event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('w'),
                KeyModifiers::NONE,
                kind,
            )));
        }
        assert!(!g.keys.w.held(true));
    }

    #[test]
    fn enhanced_input_holds_simultaneous_keys_until_real_release() {
        let mut g = fixture();
        g.configure_input(true, true);
        for code in [KeyCode::Char('w'), KeyCode::Char('d'), KeyCode::Right] {
            g.input_event(Event::Key(KeyEvent::new_with_kind(
                code,
                KeyModifiers::NONE,
                KeyEventKind::Press,
            )));
        }
        g.keys.w.at -= Duration::from_secs(2);
        assert!(g.keys.w.held(g.keys.saw_release));
        g.input_event(Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('w'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        )));
        assert!(!g.keys.w.held(true));
        assert!(g.keys.d.held(true));
        assert!(g.keys.right.held(true));
        g.input_event(Event::FocusLost);
        assert!(!g.keys.d.held(true));
        assert!(!g.keys.right.held(true));
        assert!(g.is_paused());
    }

    #[test]
    fn relative_look_has_correct_axes_and_pauses_without_drift() {
        let mut g = fixture();
        g.configure_input(true, true);
        let before = g.player.yaw;
        g.relative_look(200.0, 0.0);
        assert!(g.player.yaw > before, "mouse right must turn right");
        g.input_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(g.is_paused());
        let paused = g.player.yaw;
        g.relative_look(400.0, 50.0);
        assert_eq!(g.player.yaw, paused);
        g.input_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(!g.is_paused());
        g.relative_look(5000.0, 0.0);
        assert!(
            g.player.yaw - paused > std::f32::consts::TAU,
            "relative look must not hit a window edge"
        );
    }

    #[test]
    fn frame_pacing_targets_sixty_and_blocked_steps_do_not_bob() {
        assert!(frame_budget(false) >= Duration::from_micros(16_600));
        assert!(frame_budget(false) < Duration::from_millis(17));
        assert!(frame_budget(true) > frame_budget(false));
        let mut g = fixture();
        let mut before = vec![0; 320 * 200 * 4];
        g.view.draw(&mut before, 320, 200, Block::Terra);
        g.step_player(0.05, 0.0, 1.0, false);
        let mut after = vec![0; before.len()];
        g.view.draw(&mut after, 320, 200, Block::Terra);
        assert!(
            before == after,
            "pressing into a wall must not produce walking bob"
        );
    }

    #[test]
    fn quick_click_still_swings_when_released_before_next_frame() {
        let mut g = fixture();
        let mut idle = vec![0; 320 * 200 * 4];
        g.view.draw(&mut idle, 320, 200, Block::Terra);
        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
        ] {
            g.handle_mouse(MouseEvent {
                kind,
                column: 10,
                row: 10,
                modifiers: KeyModifiers::NONE,
            });
        }
        g.view.advance(0.1, false);
        let mut active = vec![0; idle.len()];
        g.view.draw(&mut active, 320, 200, Block::Terra);
        assert!(active != idle, "quick clicks must animate the hand");
    }

    #[test]
    fn blocked_placement_does_not_spend_inventory() {
        let mut g = fixture();
        g.place_block();
        assert_eq!(g.inv.terra, 5);
        assert_eq!(g.world.get(20, 21, 20), Block::Air);
        assert_eq!(g.toast, "BLOCKED");
    }

    #[test]
    fn tool_keys_and_block_slots_equip_the_viewmodel() {
        let mut g = fixture();
        for (key, tool) in [
            ('7', Tool::Pickaxe),
            ('8', Tool::Axe),
            ('9', Tool::Shovel),
            ('0', Tool::Hand),
            ('2', Tool::Block),
        ] {
            g.handle_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE));
            assert_eq!(g.view.tool, tool);
        }
        assert_eq!(g.slot, 1);
    }

    #[test]
    fn holding_mines_over_time_and_release_cancels() {
        let mut g = fixture();
        g.view.tool = Tool::Pickaxe;
        g.lmb = true;
        g.tick_mining(0.05);
        assert_eq!(g.world.get(21, 21, 20), Block::Slate);
        assert!(g.mine_progress > 0.0);
        g.lmb = false;
        g.tick_mining(0.05);
        assert_eq!(g.mine_progress, 0.0);
        g.lmb = true;
        for _ in 0..10 {
            g.tick_mining(0.05);
        }
        assert_eq!(g.world.get(21, 21, 20), Block::Air);
        assert_eq!(g.inv.slate, 1);
    }

    #[test]
    fn map_and_help_do_not_mine_in_background() {
        let mut g = fixture();
        g.lmb = true;
        g.map = true;
        for _ in 0..30 {
            g.tick_mining(0.05);
        }
        assert_eq!(g.world.get(21, 21, 20), Block::Slate);
        assert_eq!(g.mine_progress, 0.0);
    }

    #[test]
    fn failed_save_is_reported_not_claimed_saved() {
        let mut g = fixture();
        g.save_path = std::env::temp_dir(); // a directory, never a valid world file
        g.save_toast();
        assert_eq!(g.toast, "SAVE FAILED");
    }
}
