// Numeric camera/rectangle APIs intentionally use explicit coordinate arguments.
#![allow(clippy::too_many_arguments)]
use crate::block::{Inventory, HOTBAR};
use crate::kitty::Metrics;
use crate::kitty_gfx::Presenter;
use crate::player::{raycast, Player};
use crate::viewmodel::ViewModel;
use crate::world::{World, SX, SZ};

const FOG_NEAR: f32 = 18.0;
const FOG_FAR: f32 = 42.0;
const SKY_TOP: (u8, u8, u8) = (70, 99, 155);
const SKY_HORIZON: (u8, u8, u8) = (232, 160, 92);
const FOG: (u8, u8, u8) = (209, 154, 102);
const HUD_ROWS: i32 = 3;
pub(crate) const TEXT: (u8, u8, u8) = (239, 230, 214);
pub(crate) const DIM: (u8, u8, u8) = (138, 133, 120);
pub(crate) const AMBER: (u8, u8, u8) = (255, 180, 84);
const FAIL: (u8, u8, u8) = (255, 106, 94);
const PANEL: (u8, u8, u8) = (14, 11, 8);

pub(crate) fn term_metrics(cols: u16, rows: u16) -> Metrics {
    use std::io;
    use std::os::fd::AsRawFd;
    Metrics::from_fd(io::stdout().as_raw_fd()).unwrap_or(Metrics {
        cols,
        rows,
        cell_w: 8,
        cell_h: 16,
        win_w: cols as u32 * 8,
        win_h: rows as u32 * 16,
    })
}

pub struct Frame {
    pub cols: u16,
    pub rows: u16,
    buf: Vec<u8>,
    rgba: Vec<u8>,
    presenter: Presenter,
}

impl Frame {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            buf: Vec::with_capacity((cols as usize) * (rows as usize) * 28),
            rgba: Vec::new(),
            presenter: Presenter::new(),
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols.max(40);
        self.rows = rows.max(16);
        self.buf.clear();
    }

    pub fn draw(
        &mut self,
        world: &World,
        player: &Player,
        inv: &Inventory,
        slot: usize,
        creative: bool,
        toast: &str,
        fps: u32,
        debug: bool,
        map: bool,
        pixels: bool,
        deny: bool,
        metrics: Metrics,
        view: &ViewModel,
        mining: f32,
        help: bool,
    ) {
        self.buf.clear();
        // home + hide cursor
        self.buf.extend_from_slice(b"\x1b[?25l\x1b[H");

        let cols = self.cols as i32;
        let rows = self.rows as i32;
        let view_rows = if pixels && !map {
            rows
        } else {
            (rows - HUD_ROWS).max(4)
        };

        if map {
            crate::kitty_gfx::delete_all(&mut self.buf);
            self.draw_map(world, player, cols, view_rows);
            self.draw_hud(
                world, player, inv, slot, creative, toast, fps, debug, deny, cols,
            );
        } else if pixels {
            self.draw_world_pixels(
                world, player, inv, slot, creative, toast, fps, debug, deny, metrics, view, mining,
                help,
            );
        } else {
            self.draw_world(world, player, cols, view_rows * 2, view, HOTBAR[slot]);
            self.draw_hud(
                world, player, inv, slot, creative, toast, fps, debug, deny, cols,
            );
        }
        if help && (!pixels || map) {
            use std::io::Write;
            for (i, line) in HELP_LINES.iter().enumerate() {
                let _ = write!(
                    self.buf,
                    "\x1b[{};2H\x1b[0;37;40m{:<width$}",
                    i + 2,
                    line,
                    width = (cols - 3).max(1) as usize
                );
            }
        }
    }

    fn draw_world(
        &mut self,
        world: &World,
        player: &Player,
        cols: i32,
        samples_y: i32,
        view: &ViewModel,
        block: crate::block::Block,
    ) {
        let (ox, oy, oz) = player.eye();
        let (ldx, ldy, ldz) = player.look_dir();
        // camera basis
        let len = (ldx * ldx + ldz * ldz).sqrt().max(1e-5);
        let fx = ldx / len;
        let fz = ldz / len;
        let rx = -fz;
        let rz = fx;
        let ux = 0.0;
        let uy = 1.0;
        let uz = 0.0;

        let fov = 1.15; // ~66 deg
        let aspect = cols as f32 / (samples_y as f32 * 0.5);
        let mut pixels: Vec<(u8, u8, u8)> = vec![(0, 0, 0); (cols * samples_y) as usize];

        for sy in 0..samples_y {
            let ny = 1.0 - ((sy as f32 + 0.5) / samples_y as f32) * 2.0;
            for sx in 0..cols {
                let nx = ((sx as f32 + 0.5) / cols as f32) * 2.0 - 1.0;
                let mut dx = fx + rx * nx * fov * aspect + ux * ny * fov;
                let mut dy = ldy + uy * ny * fov;
                let mut dz = fz + rz * nx * fov * aspect + uz * ny * fov;
                let inv = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-6);
                dx /= inv;
                dy /= inv;
                dz /= inv;

                let color = sample(world, ox, oy, oz, dx, dy, dz);
                pixels[(sy * cols + sx) as usize] = color;
            }
        }

        let mut rgba: Vec<u8> = pixels
            .iter()
            .flat_map(|&(r, g, b)| [r, g, b, 255])
            .collect();
        view.draw(&mut rgba, cols, samples_y, block);
        for (p, c) in pixels.iter_mut().zip(rgba.as_chunks::<4>().0.iter()) {
            *p = (c[0], c[1], c[2]);
        }
        // half-block pack: two samples → one cell
        let view_rows = samples_y / 2;
        for row in 0..view_rows {
            for col in 0..cols {
                let top = pixels[(row * 2 * cols + col) as usize];
                let bot = pixels[((row * 2 + 1) * cols + col) as usize];
                // crosshair
                if col == cols / 2 && row == view_rows / 2 {
                    push_half(&mut self.buf, (239, 230, 214), bot);
                } else {
                    push_half(&mut self.buf, top, bot);
                }
            }
            self.buf.extend_from_slice(b"\x1b[0m\r\n");
        }
    }

    fn draw_world_pixels(
        &mut self,
        world: &World,
        player: &Player,
        inv: &Inventory,
        slot: usize,
        creative: bool,
        toast: &str,
        fps: u32,
        debug: bool,
        deny: bool,
        metrics: Metrics,
        view: &ViewModel,
        mining: f32,
        help: bool,
    ) {
        let (pw, ph) = metrics.view_px(0);
        let pw = pw as i32;
        let ph = ph as i32;
        let (ox, oy, oz) = player.eye();
        let (yaw, pitch) = (player.yaw, player.pitch);
        let need = (pw * ph * 4) as usize;
        if self.rgba.len() != need {
            self.rgba.resize(need, 0);
        }
        fill_view(&mut self.rgba, pw, ph, world, ox, oy, oz, yaw, pitch, true);
        view.draw(&mut self.rgba, pw, ph, HOTBAR[slot.min(5)]);
        let label = format!("{}   H HELP   I INVENTORY", view.tool.name());
        blit_text_px(&mut self.rgba, pw, ph, 12, ph - 20, &label, DIM, 1.0);
        if mining > 0.0 {
            let w = (pw / 8).max(24);
            let x = (pw - w) / 2;
            let y = ph / 2 + 18;
            fill_rect(&mut self.rgba, pw, ph, x, y, w, 6, PANEL);
            fill_rect(
                &mut self.rgba,
                pw,
                ph,
                x,
                y,
                (w as f32 * mining.clamp(0.0, 1.0)) as i32,
                6,
                AMBER,
            );
        }
        overlay_hud(
            &mut self.rgba,
            pw,
            ph,
            world,
            player,
            inv,
            slot,
            creative,
            toast,
            fps,
            debug,
            deny,
        );
        if help {
            overlay_help(&mut self.rgba, pw, ph, inv);
        }
        let mut tmp = Vec::new();
        let _ = self
            .presenter
            .present(&mut tmp, &self.rgba, pw as u32, ph as u32);
        self.buf.extend_from_slice(&tmp);
    }

    fn draw_map(&mut self, world: &World, player: &Player, cols: i32, view_rows: i32) {
        let scale = 1i32;
        let cx = player.x as i32;
        let cz = player.z as i32;
        for row in 0..view_rows {
            for col in 0..cols {
                let wx = cx + (col - cols / 2) * scale;
                let wz = cz + (row - view_rows / 2) * scale;
                let (r, g, b) = if wx == cx && wz == cz {
                    (255, 180, 84)
                } else if wx < 0 || wz < 0 || wx >= SX || wz >= SZ {
                    (23, 19, 16)
                } else {
                    let y = world.surface_y(wx, wz);
                    let b = world.get(wx, y, wz);
                    b.rgb(0)
                };
                push_cell(&mut self.buf, r, g, b, b" ");
            }
            self.buf.extend_from_slice(b"\x1b[0m\r\n");
        }
    }

    fn draw_hud(
        &mut self,
        world: &World,
        player: &Player,
        inv: &Inventory,
        slot: usize,
        creative: bool,
        toast: &str,
        fps: u32,
        debug: bool,
        deny: bool,
        cols: i32,
    ) {
        let cols = cols.max(1) as usize;
        let mut rows = [
            vec![HudCell::empty(); cols],
            vec![HudCell::empty(); cols],
            vec![HudCell::empty(); cols],
        ];

        if debug {
            let lang = world.lang_at(player.x as i32, player.z as i32);
            let mode = if creative { "CREATIVE" } else { "SURVIVAL" };
            let dbg = format!(
                " {x:.0},{y:.0},{z:.0}  {fps}  {lang}  {mode}",
                x = player.x,
                y = player.y,
                z = player.z,
            );
            blit_text(&mut rows[0], 0, &dbg, DIM, PANEL);
        }

        if !toast.is_empty() {
            let t = toast.to_string();
            let start = cols.saturating_sub(t.chars().count()) / 2;
            blit_text(&mut rows[0], start, &t, TEXT, PANEL);
        }

        // 6×2 slots, 1-cell gap, extra cell before crafted (CORE/CONDUIT/BEACON).
        const SLOT_W: usize = 6;
        let bar_w = SLOT_W * 6 + 5 + 1;
        let origin = cols.saturating_sub(bar_w) / 2;
        for (i, block) in HOTBAR.iter().enumerate() {
            let mut x = origin + i * (SLOT_W + 1);
            if i >= 3 {
                x += 1;
            }
            let selected = i == slot;
            let n = inv.held_count(*block);
            let fill = block.rgb(0);
            let frame = if selected && deny {
                FAIL
            } else if selected {
                AMBER
            } else {
                (48, 42, 36)
            };
            for dy in 0..2 {
                for dx in 0..SLOT_W {
                    let bg = if dx == 0 || dx == SLOT_W - 1 {
                        frame
                    } else {
                        fill
                    };
                    put(&mut rows[1 + dy], x + dx, ' ', TEXT, bg);
                }
            }
            put(
                &mut rows[1],
                x + 1,
                char::from(b'1' + i as u8),
                if selected { AMBER } else { DIM },
                fill,
            );
            let count = if n > 99 {
                "99".to_string()
            } else {
                n.to_string()
            };
            blit_text(
                &mut rows[2],
                x + SLOT_W - 1 - count.len(),
                &count,
                if selected { TEXT } else { DIM },
                fill,
            );
        }

        for row in &rows {
            emit_hud_row(&mut self.buf, row);
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }
}

fn push_half(buf: &mut Vec<u8>, top: (u8, u8, u8), bot: (u8, u8, u8)) {
    // ▀  fg=top bg=bot
    buf.extend_from_slice(b"\x1b[38;2;");
    itoa3(buf, top.0);
    buf.push(b';');
    itoa3(buf, top.1);
    buf.push(b';');
    itoa3(buf, top.2);
    buf.extend_from_slice(b"m\x1b[48;2;");
    itoa3(buf, bot.0);
    buf.push(b';');
    itoa3(buf, bot.1);
    buf.push(b';');
    itoa3(buf, bot.2);
    buf.extend_from_slice(b"m\xE2\x96\x80"); // ▀
}

fn push_cell(buf: &mut Vec<u8>, r: u8, g: u8, b: u8, ch: &[u8]) {
    buf.extend_from_slice(b"\x1b[48;2;");
    itoa3(buf, r);
    buf.push(b';');
    itoa3(buf, g);
    buf.push(b';');
    itoa3(buf, b);
    buf.extend_from_slice(b"m\x1b[38;2;239;230;214m");
    buf.extend_from_slice(ch);
}

#[derive(Clone, Copy)]
struct HudCell {
    ch: char,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
}

impl HudCell {
    fn empty() -> Self {
        Self {
            ch: ' ',
            fg: TEXT,
            bg: PANEL,
        }
    }
}

fn put(row: &mut [HudCell], x: usize, ch: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) {
    if x < row.len() {
        row[x] = HudCell { ch, fg, bg };
    }
}

fn blit_text(row: &mut [HudCell], x: usize, text: &str, fg: (u8, u8, u8), bg: (u8, u8, u8)) {
    for (i, ch) in text.chars().enumerate() {
        put(row, x + i, ch, fg, bg);
    }
}

fn emit_hud_row(buf: &mut Vec<u8>, row: &[HudCell]) {
    let mut last_fg = (255, 255, 255);
    let mut last_bg = (0, 0, 0);
    for cell in row {
        if cell.fg != last_fg {
            buf.extend_from_slice(b"\x1b[38;2;");
            itoa3(buf, cell.fg.0);
            buf.push(b';');
            itoa3(buf, cell.fg.1);
            buf.push(b';');
            itoa3(buf, cell.fg.2);
            buf.push(b'm');
            last_fg = cell.fg;
        }
        if cell.bg != last_bg {
            buf.extend_from_slice(b"\x1b[48;2;");
            itoa3(buf, cell.bg.0);
            buf.push(b';');
            itoa3(buf, cell.bg.1);
            buf.push(b';');
            itoa3(buf, cell.bg.2);
            buf.push(b'm');
            last_bg = cell.bg;
        }
        let mut tmp = [0u8; 4];
        buf.extend_from_slice(cell.ch.encode_utf8(&mut tmp).as_bytes());
    }
    buf.extend_from_slice(b"\x1b[0m\r\n");
}

pub(crate) fn fill_view(
    rgba: &mut [u8],
    pw: i32,
    ph: i32,
    world: &World,
    ox: f32,
    oy: f32,
    oz: f32,
    yaw: f32,
    pitch: f32,
    crosshair: bool,
) {
    if pw < 2 || ph < 2 {
        return;
    }
    let cp = pitch.cos();
    let ldx = yaw.cos() * cp;
    let ldy = pitch.sin();
    let ldz = yaw.sin() * cp;
    let len = (ldx * ldx + ldz * ldz).sqrt().max(1e-5);
    let fx = ldx / len;
    let fz = ldz / len;
    let rx = -fz;
    let rz = fx;
    let fov = 1.15;
    let aspect = pw as f32 / ph as f32;
    let cx = pw / 2;
    let cy = ph / 2;
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(1, 8);
    let band = (ph as usize).div_ceil(threads);
    std::thread::scope(|scope| {
        for (t, chunk) in rgba.chunks_mut(band * pw as usize * 4).enumerate() {
            let y0 = (t * band) as i32;
            scope.spawn(move || {
                let rows = (chunk.len() / (pw as usize * 4)) as i32;
                for ly in 0..rows {
                    let sy = y0 + ly;
                    let ny = 1.0 - ((sy as f32 + 0.5) / ph as f32) * 2.0;
                    for sx in 0..pw {
                        let nx = ((sx as f32 + 0.5) / pw as f32) * 2.0 - 1.0;
                        let mut dx = fx + rx * nx * fov * aspect;
                        let mut dy = ldy + ny * fov;
                        let mut dz = fz + rz * nx * fov * aspect;
                        let inv = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-6);
                        dx /= inv;
                        dy /= inv;
                        dz /= inv;
                        let (mut r, mut g, mut b) = sample(world, ox, oy, oz, dx, dy, dz);
                        if crosshair
                            && ((sx - cx).abs() <= 1 && (sy - cy).abs() <= 8
                                || (sy - cy).abs() <= 1 && (sx - cx).abs() <= 8)
                        {
                            r = 239;
                            g = 230;
                            b = 214;
                        }
                        let i = (ly * pw + sx) as usize * 4;
                        chunk[i] = r;
                        chunk[i + 1] = g;
                        chunk[i + 2] = b;
                        chunk[i + 3] = 255;
                    }
                }
            });
        }
    });
}

const HELP_LINES: [&str; 12] = [
    "TERMINAL CRAFT - CONTROLS AND RECIPES",
    "WASD MOVE   ARROWS OR MOUSE LOOK   SPACE JUMP",
    "HOLD LMB OR E/F TO MINE   RMB OR Q/TAB PLACE",
    "0 HAND   7 PICKAXE   8 AXE   9 SHOVEL",
    "1-6 BLOCKS   SCROLL OR BRACKETS SELECT",
    "C CREATIVE   DOUBLE SPACE FLIGHT   Z DESCEND",
    "M MAP   F3 DEBUG   R SAVE   ESC SAVE AND QUIT",
    "CORE - 8 SLATE AND 1 GEM",
    "CONDUIT - 3 SLATE AND 1 SAND",
    "BEACON - 4 SAND AND 1 GEM",
    "TOOLS ARE AVAILABLE IN BOTH GAME MODES",
    "H / I / ESC CLOSE THIS PANEL",
];

fn overlay_help(rgba: &mut [u8], pw: i32, ph: i32, inv: &Inventory) {
    let scale = ((pw as f32 / 350.0).min(ph as f32 / 270.0))
        .floor()
        .clamp(1.0, 3.0);
    let w = (320.0 * scale) as i32;
    let h = (220.0 * scale) as i32;
    let x = (pw - w) / 2;
    let y = (ph - h) / 2;
    fill_rect(rgba, pw, ph, x, y, w, h, PANEL);
    stroke_rect(rgba, pw, ph, x, y, w, h, 2, AMBER);
    for (i, line) in HELP_LINES.iter().enumerate() {
        blit_text_px(
            rgba,
            pw,
            ph,
            x + 8,
            y + 10 + i as i32 * (14.0 * scale) as i32,
            line,
            if i == 0 { AMBER } else { TEXT },
            scale,
        );
    }
    let resources = format!("TERRA {} SLATE {} WOOD {}", inv.terra, inv.slate, inv.wood);
    let rare = format!("SAND {} GEM {}", inv.sand, inv.gem);
    blit_text_px(
        rgba,
        pw,
        ph,
        x + 8,
        y + h - (32.0 * scale) as i32,
        &resources,
        AMBER,
        scale,
    );
    blit_text_px(
        rgba,
        pw,
        ph,
        x + 8,
        y + h - (18.0 * scale) as i32,
        &rare,
        AMBER,
        scale,
    );
}

pub(crate) fn darken_bottom(rgba: &mut [u8], pw: i32, ph: i32, from: f32, amt: f32) {
    let y0 = (ph as f32 * from) as i32;
    for y in y0.max(0)..ph {
        let t = (y - y0) as f32 / (ph - y0).max(1) as f32;
        let k = 1.0 - t * amt;
        for x in 0..pw {
            let i = ((y * pw + x) * 4) as usize;
            if i + 2 < rgba.len() {
                rgba[i] = (rgba[i] as f32 * k) as u8;
                rgba[i + 1] = (rgba[i + 1] as f32 * k) as u8;
                rgba[i + 2] = (rgba[i + 2] as f32 * k) as u8;
            }
        }
    }
}

fn overlay_hud(
    rgba: &mut [u8],
    pw: i32,
    ph: i32,
    world: &World,
    player: &Player,
    inv: &Inventory,
    slot: usize,
    creative: bool,
    toast: &str,
    fps: u32,
    debug: bool,
    deny: bool,
) {
    if pw < 8 || ph < 8 {
        return;
    }
    let scale = (ph as f32 / 720.0).clamp(0.55, 1.6);
    let slot_px = (48.0 * scale).round().max(28.0) as i32;
    let gap = (6.0 * scale).round().max(3.0) as i32;
    let craft_gap = (10.0 * scale).round().max(6.0) as i32;
    let bottom = (18.0 * scale).round().max(10.0) as i32;
    let border = (2.0 * scale).round().max(2.0) as i32;
    let bar_w = slot_px * 6 + gap * 5 + (craft_gap - gap);
    let x0 = (pw - bar_w) / 2;
    let y0 = ph - bottom - slot_px;

    if !toast.is_empty() {
        let tw = (toast.len() as i32) * (6.0 * scale).round().max(5.0) as i32;
        blit_text_px(
            rgba,
            pw,
            ph,
            (pw - tw) / 2,
            y0 - (12.0 * scale).round().max(8.0) as i32,
            toast,
            TEXT,
            scale,
        );
    }
    if debug {
        let lang = world.lang_at(player.x as i32, player.z as i32);
        let mode = if creative { "CREATIVE" } else { "SURVIVAL" };
        let dbg = format!(
            "{x:.0},{y:.0},{z:.0}  {fps}  {lang}  {mode}",
            x = player.x,
            y = player.y,
            z = player.z,
        );
        blit_text_px(rgba, pw, ph, 12, 12, &dbg, DIM, scale);
    }

    for (i, block) in HOTBAR.iter().enumerate() {
        let mut x = x0 + (i as i32) * (slot_px + gap);
        if i >= 3 {
            x += craft_gap - gap;
        }
        let selected = i == slot;
        let lift = if selected {
            (5.0 * scale).round() as i32
        } else {
            0
        };
        let y = y0 - lift;
        let fill = block.rgb(0);
        let frame = if selected && deny {
            FAIL
        } else if selected {
            AMBER
        } else {
            (70, 62, 54)
        };
        fill_rect(rgba, pw, ph, x, y, slot_px, slot_px, (14, 11, 8));
        stroke_rect(rgba, pw, ph, x, y, slot_px, slot_px, border, frame);
        let inner = slot_px - border * 2 - 4;
        if inner > 4 {
            fill_rect(
                rgba,
                pw,
                ph,
                x + border + 2,
                y + border + 2,
                inner,
                inner,
                fill,
            );
        }
        let gs = (scale * 1.6).clamp(1.0, 3.0);
        blit_text_px(
            rgba,
            pw,
            ph,
            x + border + 2,
            y + border + 1,
            &format!("{}", i + 1),
            if selected { AMBER } else { DIM },
            gs,
        );
        let n = inv.held_count(*block);
        let count = if n > 99 {
            "99".to_string()
        } else {
            n.to_string()
        };
        let cw = (count.len() as i32) * (5.0 * gs).round() as i32;
        blit_text_px(
            rgba,
            pw,
            ph,
            x + slot_px - border - 2 - cw,
            y + slot_px - border - (7.0 * gs).round() as i32,
            &count,
            TEXT,
            gs,
        );
    }
}

pub(crate) fn fill_rect(
    rgba: &mut [u8],
    pw: i32,
    ph: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    c: (u8, u8, u8),
) {
    for yy in y.max(0)..(y + h).min(ph) {
        for xx in x.max(0)..(x + w).min(pw) {
            put_px(rgba, pw, xx, yy, c);
        }
    }
}

pub(crate) fn stroke_rect(
    rgba: &mut [u8],
    pw: i32,
    ph: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    t: i32,
    c: (u8, u8, u8),
) {
    fill_rect(rgba, pw, ph, x, y, w, t, c);
    fill_rect(rgba, pw, ph, x, y + h - t, w, t, c);
    fill_rect(rgba, pw, ph, x, y, t, h, c);
    fill_rect(rgba, pw, ph, x + w - t, y, t, h, c);
}

fn put_px(rgba: &mut [u8], pw: i32, x: i32, y: i32, c: (u8, u8, u8)) {
    if x < 0 || y < 0 {
        return;
    }
    let i = ((y * pw + x) * 4) as usize;
    if i + 3 < rgba.len() {
        rgba[i] = c.0;
        rgba[i + 1] = c.1;
        rgba[i + 2] = c.2;
        rgba[i + 3] = 255;
    }
}

pub(crate) fn blit_text_px(
    rgba: &mut [u8],
    pw: i32,
    ph: i32,
    x: i32,
    y: i32,
    text: &str,
    c: (u8, u8, u8),
    scale: f32,
) {
    let s = scale.round().max(1.0) as i32;
    let mut cx = x;
    for ch in text.chars() {
        if let Some(g) = glyph(ch) {
            for row in 0..7 {
                for col in 0..5 {
                    let bit = 34 - (row * 5 + col);
                    if (g >> bit) & 1 == 1 {
                        fill_rect(rgba, pw, ph, cx + col * s, y + row * s, s, s, c);
                    }
                }
            }
            cx += 6 * s;
        } else {
            cx += 4 * s;
        }
    }
}

/// 5×7 glyphs, bit 0 = top-left, row-major, MSB of each row is left.
fn glyph(ch: char) -> Option<u64> {
    let bits: u64 = match ch.to_ascii_uppercase() {
        '0' => 0b01110_10001_10001_10001_10001_10001_01110,
        '1' => 0b00100_01100_00100_00100_00100_00100_01110,
        '2' => 0b01110_10001_00001_00010_00100_01000_11111,
        '3' => 0b01110_10001_00001_00110_00001_10001_01110,
        '4' => 0b00010_00110_01010_10010_11111_00010_00010,
        '5' => 0b11111_10000_11110_00001_00001_10001_01110,
        '6' => 0b01110_10000_10000_11110_10001_10001_01110,
        '7' => 0b11111_00001_00010_00100_01000_01000_01000,
        '8' => 0b01110_10001_10001_01110_10001_10001_01110,
        '9' => 0b01110_10001_10001_01111_00001_00001_01110,
        'A' => 0b01110_10001_10001_11111_10001_10001_10001,
        'B' => 0b11110_10001_10001_11110_10001_10001_11110,
        'C' => 0b01110_10001_10000_10000_10000_10001_01110,
        'D' => 0b11110_10001_10001_10001_10001_10001_11110,
        'E' => 0b11111_10000_10000_11110_10000_10000_11111,
        'F' => 0b11111_10000_10000_11110_10000_10000_10000,
        'G' => 0b01110_10001_10000_10111_10001_10001_01110,
        'H' => 0b10001_10001_10001_11111_10001_10001_10001,
        'I' => 0b01110_00100_00100_00100_00100_00100_01110,
        'J' => 0b00111_00010_00010_00010_00010_10010_01100,
        'K' => 0b10001_10010_10100_11000_10100_10010_10001,
        'L' => 0b10000_10000_10000_10000_10000_10000_11111,
        'M' => 0b10001_11011_10101_10101_10001_10001_10001,
        'N' => 0b10001_11001_10101_10011_10001_10001_10001,
        'O' => 0b01110_10001_10001_10001_10001_10001_01110,
        'P' => 0b11110_10001_10001_11110_10000_10000_10000,
        'Q' => 0b01110_10001_10001_10001_10101_10010_01101,
        'R' => 0b11110_10001_10001_11110_10100_10010_10001,
        'S' => 0b01110_10001_10000_01110_00001_10001_01110,
        'T' => 0b11111_00100_00100_00100_00100_00100_00100,
        'U' => 0b10001_10001_10001_10001_10001_10001_01110,
        'V' => 0b10001_10001_10001_10001_10001_01010_00100,
        'W' => 0b10001_10001_10001_10101_10101_10101_01010,
        'X' => 0b10001_10001_01010_00100_01010_10001_10001,
        'Y' => 0b10001_10001_01010_00100_00100_00100_00100,
        'Z' => 0b11111_00001_00010_00100_01000_10000_11111,
        '-' => 0b00000_00000_00000_11111_00000_00000_00000,
        ',' => 0b00000_00000_00000_00000_00100_00100_01000,
        '!' => 0b00100_00100_00100_00100_00100_00000_00100,
        '.' => 0b00000_00000_00000_00000_00000_00100_00100,
        _ => return None,
    };
    Some(bits)
}

fn itoa3(buf: &mut Vec<u8>, v: u8) {
    if v >= 100 {
        buf.push(b'0' + v / 100);
        buf.push(b'0' + (v / 10) % 10);
        buf.push(b'0' + v % 10);
    } else if v >= 10 {
        buf.push(b'0' + v / 10);
        buf.push(b'0' + v % 10);
    } else {
        buf.push(b'0' + v);
    }
}

fn sample(world: &World, ox: f32, oy: f32, oz: f32, dx: f32, dy: f32, dz: f32) -> (u8, u8, u8) {
    if let Some(hit) = raycast(world, ox, oy, oz, dx, dy, dz, FOG_FAR) {
        let (mut r, mut g, mut b) = hit.block.rgb(hit.face);
        let hx = ox + dx * hit.dist;
        let hy = oy + dy * hit.dist;
        let hz = oz + dz * hit.dist;
        let (u, v) = match hit.face {
            0 | 1 => (hx.fract().abs(), hz.fract().abs()),
            2 | 3 => (hz.fract().abs(), hy.fract().abs()),
            _ => (hx.fract().abs(), hy.fract().abs()),
        };
        // 8×8 texel noise so faces read as blocks, not flat fills
        let tex = (((u * 8.0) as i32).wrapping_mul(31) ^ ((v * 8.0) as i32).wrapping_mul(17)) & 7;
        let delta = tex as i16 - 3;
        r = (r as i16 + delta).clamp(0, 255) as u8;
        g = (g as i16 + delta).clamp(0, 255) as u8;
        b = (b as i16 + delta).clamp(0, 255) as u8;
        fog(r, g, b, hit.dist)
    } else {
        sky(dy)
    }
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0)) as u8
}

fn sky(dy: f32) -> (u8, u8, u8) {
    let t = ((dy + 0.15) * 0.9).clamp(0.0, 1.0);
    (
        lerp(SKY_HORIZON.0, SKY_TOP.0, t),
        lerp(SKY_HORIZON.1, SKY_TOP.1, t),
        lerp(SKY_HORIZON.2, SKY_TOP.2, t),
    )
}

fn fog(r: u8, g: u8, b: u8, dist: f32) -> (u8, u8, u8) {
    if dist <= FOG_NEAR {
        return (r, g, b);
    }
    let t = ((dist - FOG_NEAR) / (FOG_FAR - FOG_NEAR)).clamp(0.0, 1.0);
    (lerp(r, FOG.0, t), lerp(g, FOG.1, t), lerp(b, FOG.2, t))
}
