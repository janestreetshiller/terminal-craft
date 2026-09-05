//! Device-pixel geometry. Same ioctl path as termin8-command-palette `geom.rs`.

use std::io::{self, Read, Write};
use std::os::fd::RawFd;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub cols: u16,
    pub rows: u16,
    pub cell_w: u16,
    pub cell_h: u16,
    pub win_w: u32,
    pub win_h: u32,
}

impl Metrics {
    pub fn from_fd(fd: RawFd) -> io::Result<Self> {
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        if ws.ws_col == 0 || ws.ws_row == 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "empty winsize"));
        }
        if ws.ws_xpixel > 0 && ws.ws_ypixel > 0 {
            return Ok(Self {
                cols: ws.ws_col,
                rows: ws.ws_row,
                cell_w: (ws.ws_xpixel / ws.ws_col).max(1),
                cell_h: (ws.ws_ypixel / ws.ws_row).max(1),
                win_w: ws.ws_xpixel as u32,
                win_h: ws.ws_ypixel as u32,
            });
        }
        // CSI 16 t / 14 t fallback (raw mode required).
        query_csi(ws.ws_col, ws.ws_row)
    }

    pub fn view_px(&self, hud_rows: u16) -> (u32, u32) {
        let hud = self.cell_h as u32 * hud_rows as u32;
        (self.win_w.max(1), self.win_h.saturating_sub(hud).max(1))
    }
}

fn query_csi(cols: u16, rows: u16) -> io::Result<Metrics> {
    let mut tty = io::stdout();
    tty.write_all(b"\x1b[16t\x1b[14t")?;
    tty.flush()?;
    let mut stdin = io::stdin();
    let mut buf = Vec::with_capacity(64);
    let deadline = Instant::now() + Duration::from_millis(80);
    let mut chunk = [0u8; 64];
    let mut cell = None;
    let mut win = None;
    while Instant::now() < deadline && (cell.is_none() || win.is_none()) {
        match stdin.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                let s = String::from_utf8_lossy(&buf);
                if cell.is_none() {
                    if let Some(v) = parse_t(&s, '6') {
                        cell = Some(v);
                    }
                }
                if win.is_none() {
                    if let Some(v) = parse_t(&s, '4') {
                        win = Some(v);
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(e) => return Err(e),
        }
    }
    let (ch, cw) = cell.unwrap_or((16, 8));
    let (wh, ww) = win.unwrap_or((ch * rows as u32, cw * cols as u32));
    Ok(Metrics {
        cols,
        rows,
        cell_w: cw.max(1) as u16,
        cell_h: ch.max(1) as u16,
        win_w: ww.max(1),
        win_h: wh.max(1),
    })
}

fn parse_t(s: &str, kind: char) -> Option<(u32, u32)> {
    // CSI {kind} ; h ; w t
    let needle = format!("\x1b[{kind};");
    let i = s.find(&needle)?;
    let rest = &s[i + needle.len()..];
    let end = rest.find('t')?;
    let mut it = rest[..end].split(';');
    let h = it.next()?.parse().ok()?;
    let w = it.next()?.parse().ok()?;
    Some((h, w))
}
