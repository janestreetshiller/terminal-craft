//! Title: kitty-rendered world (slow orbit) + pixel wordmark + stacked buttons.
//! Same hierarchy as OG Minecraft / the WebGL SANDBOX screen — not a cell dirt field.

use crate::game::Game;
use crate::kitty_gfx::Presenter;
use crate::render::{
    blit_text_px, darken_bottom, fill_rect, fill_view, stroke_rect, term_metrics, AMBER, DIM, TEXT,
};
use crate::world::World;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::queue;
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};

const SPLASH: [&str; 8] = [
    "python prairie!",
    "also try zen!",
    "mine terra, craft CORE",
    "ruby rails!",
    "a voxel sandbox",
    "typescript topanga",
    "wasd + mouse",
    "100% dirt",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Continue,
    NewMap,
    Creative,
    Quit,
}

struct Btn {
    action: Action,
    label: &'static str,
    enabled: bool,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

pub fn run(out: &mut impl Write, save_path: &Path) -> io::Result<Option<Game>> {
    let has_save = save_path.exists();
    let splash = SPLASH[(seed_now() as usize) % SPLASH.len()];
    let mut selected: usize = if has_save { 0 } else { 1 };
    let world = World::generate(seed_now() ^ 0x51A7);
    let mut presenter = Presenter::new();
    let mut rgba = Vec::new();
    let mut yaw = 0.55f32;
    let mut last = Instant::now();
    let t0 = Instant::now();
    let pixels = crate::kitty_gfx::available();

    loop {
        let (cols, rows) = terminal::size()?;
        let metrics = term_metrics(cols, rows);
        let dt = last.elapsed().as_secs_f32().min(0.05);
        last = Instant::now();
        yaw += dt * 0.12;

        let (pw, ph) = (metrics.win_w.max(2) as i32, metrics.win_h.max(2) as i32);
        let need = (pw * ph * 4) as usize;
        if rgba.len() != need {
            rgba.resize(need, 0);
        }

        let (sx, sy, sz) = world.spawn();
        let ox = sx - yaw.cos() * 22.0;
        let oz = sz - yaw.sin() * 22.0;
        let oy = sy + 9.0;
        let pitch = -0.38;

        if pixels {
            fill_view(&mut rgba, pw, ph, &world, ox, oy, oz, yaw, pitch, false);
            darken_bottom(&mut rgba, pw, ph, 0.42, 0.55);
            let btns = layout_buttons(has_save, pw, ph);
            overlay_chrome(&mut rgba, pw, ph, splash, selected, &btns, t0.elapsed());
            let mut tmp = Vec::new();
            let _ = presenter.present(&mut tmp, &rgba, pw as u32, ph as u32);
            out.write_all(b"\x1b[?25l\x1b[H")?;
            out.write_all(&tmp)?;
            out.flush()?;
            if let Some(g) = pump(save_path, has_save, &btns, &mut selected, metrics.cell_w, metrics.cell_h, cols, rows)? {
                return Ok(g);
            }
        } else {
            // half-block fallback still uses the world, not a dirt field
            let frame_btns = layout_buttons(has_save, cols as i32 * 8, rows as i32 * 16);
            paint_half(out, cols, rows, &world, ox, oy, oz, yaw, pitch, splash, selected, &frame_btns)?;
            if let Some(g) = pump(save_path, has_save, &frame_btns, &mut selected, 8, 16, cols, rows)? {
                return Ok(g);
            }
        }
    }
}

fn layout_buttons(has_save: bool, pw: i32, ph: i32) -> [Btn; 4] {
    let scale = (ph as f32 / 720.0).clamp(0.6, 1.5);
    let bw = (280.0 * scale) as i32;
    let bh = (36.0 * scale) as i32;
    let gap = (10.0 * scale) as i32;
    let x = (pw - bw) / 2;
    let y0 = (ph as f32 * 0.58) as i32;
    [
        Btn { action: Action::Continue, label: "CONTINUE", enabled: has_save, x, y: y0, w: bw, h: bh },
        Btn { action: Action::NewMap, label: "NEW MAP", enabled: true, x, y: y0 + bh + gap, w: bw, h: bh },
        Btn { action: Action::Creative, label: "CREATIVE", enabled: true, x, y: y0 + (bh + gap) * 2, w: bw, h: bh },
        Btn { action: Action::Quit, label: "QUIT GAME", enabled: true, x, y: y0 + (bh + gap) * 3, w: bw, h: bh },
    ]
}

fn overlay_chrome(
    rgba: &mut [u8],
    pw: i32,
    ph: i32,
    splash: &str,
    selected: usize,
    btns: &[Btn; 4],
    elapsed: Duration,
) {
    let scale = (ph as f32 / 720.0).clamp(0.6, 1.5);
    let title_s = (scale * 5.2).clamp(3.0, 8.0);
    let sub_s = (scale * 1.6).clamp(1.0, 3.0);
    let tw = 7 * (6.0 * title_s) as i32;
    blit_text_px(rgba, pw, ph, (pw - tw) / 2 + 3, (ph as f32 * 0.16) as i32 + 3, "SANDBOX", (40, 22, 10), title_s);
    blit_text_px(rgba, pw, ph, (pw - tw) / 2, (ph as f32 * 0.16) as i32, "SANDBOX", AMBER, title_s);
    let mw = 15 * (6.0 * sub_s) as i32;
    blit_text_px(
        rgba,
        pw,
        ph,
        (pw - mw) / 2,
        (ph as f32 * 0.16) as i32 - (10.0 * sub_s) as i32,
        "MULTIPLEXERVERSE",
        DIM,
        sub_s,
    );
    let pulse = ((elapsed.as_millis() / 420) % 2) == 0;
    let sw = splash.len() as i32 * (6.0 * sub_s) as i32;
    blit_text_px(
        rgba,
        pw,
        ph,
        (pw - sw) / 2,
        (ph as f32 * 0.16) as i32 + (8.0 * title_s) as i32,
        splash,
        if pulse { AMBER } else { TEXT },
        sub_s,
    );

    for (i, b) in btns.iter().enumerate() {
        let hot = i == selected && b.enabled;
        let bg = if !b.enabled {
            (18, 14, 10)
        } else if hot {
            (72, 48, 20)
        } else {
            (28, 20, 14)
        };
        let frame = if hot { AMBER } else { (90, 64, 32) };
        let fg = if b.enabled { TEXT } else { DIM };
        fill_rect(rgba, pw, ph, b.x, b.y, b.w, b.h, bg);
        stroke_rect(rgba, pw, ph, b.x, b.y, b.w, b.h, 2.max((scale * 2.0) as i32), frame);
        let ls = (scale * 2.0).clamp(1.0, 3.0);
        let lw = b.label.len() as i32 * (6.0 * ls) as i32;
        blit_text_px(
            rgba,
            pw,
            ph,
            b.x + (b.w - lw) / 2,
            b.y + (b.h - (7.0 * ls) as i32) / 2,
            b.label,
            fg,
            ls,
        );
    }
}

fn paint_half(
    out: &mut impl Write,
    cols: u16,
    rows: u16,
    world: &World,
    ox: f32,
    oy: f32,
    oz: f32,
    yaw: f32,
    pitch: f32,
    splash: &str,
    selected: usize,
    _btns: &[Btn; 4],
) -> io::Result<()> {
    let pw = cols as i32;
    let ph = rows as i32 * 2;
    let mut rgba = vec![0u8; (pw * ph * 4) as usize];
    fill_view(&mut rgba, pw, ph, world, ox, oy, oz, yaw, pitch, false);
    let mut buf = b"\x1b[?25l\x1b[H".to_vec();
    for row in 0..rows as i32 {
        for col in 0..pw {
            let i0 = ((row * 2 * pw + col) * 4) as usize;
            let i1 = (((row * 2 + 1) * pw + col) * 4) as usize;
            let top = (rgba[i0], rgba[i0 + 1], rgba[i0 + 2]);
            let bot = (rgba[i1], rgba[i1 + 1], rgba[i1 + 2]);
            buf.extend_from_slice(b"\x1b[38;2;");
            itoa(&mut buf, top.0);
            buf.push(b';');
            itoa(&mut buf, top.1);
            buf.push(b';');
            itoa(&mut buf, top.2);
            buf.extend_from_slice(b"m\x1b[48;2;");
            itoa(&mut buf, bot.0);
            buf.push(b';');
            itoa(&mut buf, bot.1);
            buf.push(b';');
            itoa(&mut buf, bot.2);
            buf.extend_from_slice(b"m\xE2\x96\x80");
        }
        buf.extend_from_slice(b"\x1b[0m\r\n");
    }
    let _ = (splash, selected);
    out.write_all(&buf)?;
    out.flush()
}

fn pump(
    save_path: &Path,
    has_save: bool,
    btns: &[Btn; 4],
    selected: &mut usize,
    cell_w: u16,
    cell_h: u16,
    cols: u16,
    rows: u16,
) -> io::Result<Option<Option<Game>>> {
    if !event::poll(Duration::from_millis(0))? {
        return Ok(None);
    }
    match event::read()? {
        Event::Resize(..) => {
            let mut out = io::stdout();
            queue!(out, Clear(ClearType::All))?;
            Ok(None)
        }
        Event::Key(k) if k.kind == KeyEventKind::Release => Ok(None),
        Event::Key(k) => match k.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Ok(Some(None)),
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                *selected = prev_enabled(*selected, btns);
                Ok(None)
            }
            KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                *selected = next_enabled(*selected, btns);
                Ok(None)
            }
            KeyCode::Enter | KeyCode::Char(' ') => activate(btns[*selected].action, save_path),
            KeyCode::Char('1') if has_save => Ok(Some(Some(Game::load(save_path)?))),
            KeyCode::Char('1') | KeyCode::Char('2') => {
                Ok(Some(Some(Game::new_map(seed_now(), false, save_path.to_path_buf()))))
            }
            KeyCode::Char('3') => Ok(Some(Some(Game::new_map(seed_now(), true, save_path.to_path_buf())))),
            _ => Ok(None),
        },
        Event::Mouse(m) => {
            let (mx, my) = mouse_px(m.column, m.row, cell_w, cell_h, cols, rows);
            if let Some(i) = btns.iter().position(|b| {
                b.enabled && mx >= b.x && mx < b.x + b.w && my >= b.y && my < b.y + b.h
            }) {
                *selected = i;
                if matches!(m.kind, MouseEventKind::Down(MouseButton::Left)) {
                    return activate(btns[i].action, save_path);
                }
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn mouse_px(col: u16, row: u16, cell_w: u16, cell_h: u16, cols: u16, rows: u16) -> (i32, i32) {
    if col >= cols || row >= rows {
        (col as i32, row as i32)
    } else {
        (col as i32 * cell_w as i32, row as i32 * cell_h as i32)
    }
}

fn activate(action: Action, save_path: &Path) -> io::Result<Option<Option<Game>>> {
    Ok(match action {
        Action::Continue if save_path.exists() => Some(Some(Game::load(save_path)?)),
        Action::Continue => None,
        Action::NewMap => Some(Some(Game::new_map(seed_now(), false, save_path.to_path_buf()))),
        Action::Creative => Some(Some(Game::new_map(seed_now(), true, save_path.to_path_buf()))),
        Action::Quit => Some(None),
    })
}

fn next_enabled(cur: usize, btns: &[Btn]) -> usize {
    let n = btns.len();
    for k in 1..=n {
        let i = (cur + k) % n;
        if btns[i].enabled {
            return i;
        }
    }
    cur
}

fn prev_enabled(cur: usize, btns: &[Btn]) -> usize {
    let n = btns.len();
    for k in 1..=n {
        let i = (cur + n - k) % n;
        if btns[i].enabled {
            return i;
        }
    }
    cur
}

fn itoa(buf: &mut Vec<u8>, v: u8) {
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

fn seed_now() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_nanos() as u32) ^ 0xA5A5_5A5A)
        .unwrap_or(0xC0FFEE)
}
