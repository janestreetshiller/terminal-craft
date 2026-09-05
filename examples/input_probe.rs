//! Explicit, local terminal-input diagnostic; never monitors other windows.
use crossterm::{
    event::{
        self, Event, KeyCode, KeyEventKind, KeyboardEnhancementFlags as Flags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};
struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
        let _ = disable_raw_mode();
    }
}
fn main() -> io::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: input_probe OUTPUT_LOG");
    let mut log = std::fs::File::create(path)?;
    enable_raw_mode()?;
    let _guard = Restore;
    execute!(
        io::stdout(),
        PushKeyboardEnhancementFlags(
            Flags::DISAMBIGUATE_ESCAPE_CODES
                | Flags::REPORT_EVENT_TYPES
                | Flags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
        )
    )?;
    println!("Terminal Craft input probe. This window only. Esc ends; 30 second limit.\r");
    writeln!(log, "READY")?;
    log.flush()?;
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if event::poll(Duration::from_millis(100))? {
            let e = event::read()?;
            writeln!(log, "{e:?}")?;
            log.flush()?;
            if matches!(e,Event::Key(k) if k.code==KeyCode::Esc && k.kind==KeyEventKind::Press) {
                break;
            }
        }
    }
    Ok(())
}
