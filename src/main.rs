mod block;
mod config;
mod game;
mod kitty;
mod kitty_gfx;
mod menu;
mod native;
mod player;
mod render;
mod ui;
mod viewmodel;
mod world;

use crossterm::cursor::Show;
use crossterm::event::{
    DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture,
    KeyboardEnhancementFlags as Flags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
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
        let enhancement =
            kitty_input() && std::env::var_os("TERMINAL_CRAFT_LEGACY_INPUT").is_none();
        if enhancement {
            execute!(
                out,
                PushKeyboardEnhancementFlags(
                    Flags::DISAMBIGUATE_ESCAPE_CODES
                        | Flags::REPORT_EVENT_TYPES
                        | Flags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                )
            )?;
        }
        Ok(Self { enhancement })
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

pub(crate) fn kitty_input() -> bool {
    std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var("TERM").unwrap_or_default().contains("kitty")
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let theme_name = if args.first().map(String::as_str) == Some("--gilded") {
        "gilded".to_string()
    } else {
        std::env::var("TERMINAL_CRAFT_UI").unwrap_or_else(|_| "gilded".to_string())
    };
    match ui::Theme::parse(&theme_name) {
        Ok(theme) => ui::set(theme),
        Err(message) => {
            eprintln!("terminal-craft: {message}");
            return ExitCode::from(2);
        }
    }
    match args.first().map(String::as_str) {
        Some("--help" | "-h") => {
            println!("Terminal Craft {}\nNative Rust voxel sandbox — no browser or server.\n\nterminal-craft               Open the native game window (pointer lock)\nterminal-craft --gilded      Open native window with Gilded UI skin\nterminal-craft --terminal    Open optional Kitty terminal mode\nterminal-craft --here        Play in the current terminal\nterminal-craft --version     Print version\nterminal-craft --check-save PATH   Validate a save without modifying it\n\nWASD move; mouse/arrows look; Space jump/ascend; Ctrl sprint\nShift/Z sneak/descend; C creative; G or double-Space toggle flight\nHold LMB or E/F mine; RMB or Q/Tab place\n0 hand; 7 pickaxe; 8 axe; 9 shovel; 1-6 held blocks\nH/I controls and inventory; M map; F3 debug; F11 native fullscreen\nEsc pause/resume and release pointer; R save; Ctrl-Q save and quit\nF6 switches Classic / Gilded UI during this session\n\nTERMINAL_CRAFT_UI=classic|gilded selects the starting UI skin (default: gilded).\nTERMINAL_CRAFT_QUALITY=native renders at full window resolution.\nTERMINAL_CRAFT_SENS sets mouse sensitivity (default 0.0024).\nDeveloper binary: --native opens the native host; --native-smoke-test NEW_DIR runs isolated GUI QA.",env!("CARGO_PKG_VERSION"));
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
        Some("--native" | "--gilded" | "--native-smoke-test") => {
            let result = if args[0] == "--native-smoke-test" {
                if args.len() != 2 {
                    Err(io::Error::other(
                        "--native-smoke-test requires a fresh output directory",
                    ))
                } else {
                    native::run(
                        PathBuf::from(&args[1]).join("world.tcrf"),
                        Some(PathBuf::from(&args[1])),
                    )
                }
            } else {
                save_path().and_then(|p| native::run(p, None))
            };
            return match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("terminal-craft: {e}");
                    ExitCode::from(1)
                }
            };
        }
        Some("--here") => {}
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
        g.configure_input(guard.enhancement, kitty_input());
        g.run(&mut out)?;
    }
    drop(guard);
    Ok(())
}
