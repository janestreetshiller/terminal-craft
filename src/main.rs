mod block;
mod config;
mod game;
mod kitty;
mod kitty_gfx;
mod menu;
mod player;
mod render;
mod viewmodel;
mod world;

use crossterm::cursor::Show;
use crossterm::event::{
    DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture,
    PopKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
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
        execute!(
            out,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange
        )?;
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
    let _ = execute!(
        out,
        DisableMouseCapture,
        DisableFocusChange,
        Show,
        LeaveAlternateScreen
    );
    let _ = disable_raw_mode();
    let _ = execute!(out, crossterm::style::ResetColor);
    let _ = out.flush();
}

fn save_path() -> io::Result<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    config::resolve_save(
        std::env::var_os("TERMINAL_CRAFT_SAVE").map(PathBuf::from),
        std::env::var_os("TUICRAFT_SAVE").map(PathBuf::from),
        &base,
    )
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--help" | "-h") => {
            println!("Terminal Craft {}\nNative Rust voxel sandbox — no browser or server.\n\nterminal-craft             Open a dedicated kitty window\nterminal-craft --here      Play in the current terminal\nterminal-craft --version   Print version\nterminal-craft --check-save PATH   Validate a save without modifying it\n\nWASD move; mouse/arrows look; space jump\nHold LMB or E/F to mine; RMB or Q/Tab to place\n0 hand; 7 pickaxe; 8 axe; 9 shovel; 1-6 held blocks\nH/I controls, inventory and recipes; M map; F3 debug\nC creative; double-space flight; Z descend; R save; Esc save and quit",env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("--version" | "-V") => {
            println!("Terminal Craft {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("--check-save") if args.len() == 2 => {
            return match world::World::load(std::path::Path::new(&args[1])) {
                Ok((world, _, creative, _, state)) => {
                    println!(
                        "Valid Terminal Craft save: seed {}, creative {}, player state {}",
                        world.seed,
                        creative,
                        state.is_some()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("terminal-craft: {e}");
                    ExitCode::from(1)
                }
            };
        }
        Some(other) => {
            eprintln!("terminal-craft: unknown or incomplete option {other}; use --help");
            return ExitCode::from(2);
        }
        None => {}
    }

    if !io::stdout().is_terminal() || !io::stdin().is_terminal() {
        eprintln!("terminal-craft: needs an interactive terminal (open a kitty window)");
        return ExitCode::from(2);
    }

    let (cols, rows) = terminal::size().unwrap_or((0, 0));
    if cols < 80 || rows < 24 {
        eprintln!("terminal-craft: need at least 80x24 cells (got {cols}x{rows})");
        return ExitCode::from(3);
    }

    let enhancement_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let enhancement_flag = enhancement_flag.clone();
        panic::set_hook(Box::new(move |info| {
            restore(enhancement_flag.load(std::sync::atomic::Ordering::SeqCst));
            eprintln!("terminal-craft panic: {info}");
        }));
    }

    let code = match run(enhancement_flag.as_ref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.kind() == io::ErrorKind::Interrupted => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("terminal-craft: {e}");
            ExitCode::from(1)
        }
    };
    code
}

fn run(enhancement_flag: &std::sync::atomic::AtomicBool) -> io::Result<()> {
    let guard = TermGuard::enter()?;
    enhancement_flag.store(guard.enhancement, std::sync::atomic::Ordering::SeqCst);
    let mut out = io::stdout();
    let path = save_path()?;
    if let Some(mut g) = menu::run(&mut out, &path)? {
        g.run(&mut out)?;
    }
    drop(guard);
    Ok(())
}
