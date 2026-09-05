mod block;
mod game;
mod kitty;
mod kitty_gfx;
mod menu;
mod player;
mod render;
mod world;

use crossterm::cursor::Show;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, PopKeyboardEnhancementFlags};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, terminal};
use std::io::{self, IsTerminal, Write};
use std::panic;
use std::path::PathBuf;
use std::process::ExitCode;

struct TermGuard {
    enhancement: bool,
}

impl TermGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut out = io::stdout();
        execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
        // Crossterm enables 1002 (drag-only). Drop it; any-motion + SGR pixels.
        let _ = write!(out, "\x1b[?1002l\x1b[?1003h\x1b[?1006h\x1b[?1016h");
        let _ = out.flush();
        // Do not enable kitty keyboard protocol / REPORT_ALL_KEYS.
        // CSI-u swallows WASD/space as undecoded sequences in this kitty.
        Ok(Self { enhancement: false })
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        restore(self.enhancement);
    }
}

fn restore(enhancement: bool) {
    let mut out = io::stdout();
    if enhancement {
        let _ = execute!(out, PopKeyboardEnhancementFlags);
    }
    crate::kitty_gfx::delete_all(&mut out);
    let _ = write!(out, "\x1b[?1016l\x1b[?1003l");
    let _ = execute!(out, DisableMouseCapture, Show, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let _ = execute!(out, crossterm::style::ResetColor);
    let _ = out.flush();
}

fn save_path() -> PathBuf {
    if let Some(p) = std::env::var_os("TUICRAFT_SAVE") {
        return PathBuf::from(p);
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("tuicraft").join("world.tcrf")
}

fn main() -> ExitCode {
    if !io::stdout().is_terminal() || !io::stdin().is_terminal() {
        eprintln!("tuicraft: needs an interactive terminal (open a kitty window)");
        return ExitCode::from(2);
    }

    let (cols, rows) = terminal::size().unwrap_or((0, 0));
    if cols < 80 || rows < 24 {
        eprintln!("tuicraft: need at least 80x24 cells (got {cols}x{rows})");
        return ExitCode::from(3);
    }

    let enhancement_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let enhancement_flag = enhancement_flag.clone();
        panic::set_hook(Box::new(move |info| {
            restore(enhancement_flag.load(std::sync::atomic::Ordering::SeqCst));
            eprintln!("tuicraft panic: {info}");
        }));
    }

    let code = match run(enhancement_flag.as_ref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.kind() == io::ErrorKind::Interrupted => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("tuicraft: {e}");
            ExitCode::from(1)
        }
    };
    code
}

fn run(enhancement_flag: &std::sync::atomic::AtomicBool) -> io::Result<()> {
    let guard = TermGuard::enter()?;
    enhancement_flag.store(guard.enhancement, std::sync::atomic::Ordering::SeqCst);
    let mut out = io::stdout();
    let path = save_path();
    if let Some(mut g) = menu::run(&mut out, &path)? {
        g.run(&mut out)?;
    }
    drop(guard);
    Ok(())
}
