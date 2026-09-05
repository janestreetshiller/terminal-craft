//! Kitty graphics protocol wire format.
//!
//! Copied from termin8-command-palette (`src/kitty/encoder.rs`). That is the
//! proven emitter. Do not replace this with a per-frame base64 `t=d` loop.
//!
//! Wire shape: `ESC _ G <k=v,...> ; <base64 payload> ESC \`. The `a` key selects
//! the action (`t`/`T`/`p`/`d`), `f` the format, `t` the medium, `s`/`v` the
//! size, `i` the image id, and `p`/`X`/`Y`/`z`/`C` the placement. Chunked
//! sends carry the full key set on the first chunk and `m=1`/`m=0` afterwards.

use std::io::Write;

pub const APC_START: &[u8] = b"\x1b_G";
pub const APC_END: &[u8] = b"\x1b\\";

/// Max base64 characters per chunk, per spec.
pub const CHUNK: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// 24-bit RGB, no alpha.
    Rgb = 24,
    /// 32-bit straight (non-premultiplied) RGBA.
    Rgba = 32,
    Png = 100,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Medium {
    /// Base64 payload inside the escape sequence.
    Direct,
    /// Base64 POSIX shared-memory object name; kitty mmaps and unlinks it.
    SharedMem,
    /// Base64 path to a regular file kitty should delete after reading.
    TempFile,
}

impl Medium {
    fn key(self) -> u8 {
        match self {
            Medium::Direct => b'd',
            Medium::SharedMem => b's',
            Medium::TempFile => b't',
        }
    }
}

/// What to do with a transmitted image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Transmit only; place later with `Put`.
    Transmit,
    /// Transmit and place in one round trip; re-sending replaces atomically.
    TransmitAndPut,
    Put,
    Delete,
}

impl Action {
    fn key(self) -> u8 {
        match self {
            Action::Transmit => b't',
            Action::TransmitAndPut => b'T',
            Action::Put => b'p',
            Action::Delete => b'd',
        }
    }
}

/// Placement geometry. `x_off`/`y_off` are sub-cell pixel offsets and MUST be
/// smaller than one cell -- see `geom::Metrics::anchor`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Placement {
    pub id: u32,
    pub x_off: u16,
    pub y_off: u16,
    /// Draw order; >= 0 keeps the palette above cell text.
    pub z: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Transmit<'a> {
    pub id: u32,
    pub format: Format,
    pub medium: Medium,
    pub width: u32,
    pub height: u32,
    pub payload: &'a [u8],
    pub placement: Option<Placement>,
}

/// Emit a transmit (and optionally place) command. The caller positions the
/// cursor; `C=1` prevents the placement from moving it.
pub fn transmit<W: Write>(out: &mut W, t: &Transmit<'_>) -> std::io::Result<()> {
    let b64 = base64(t.payload);
    let action = if t.placement.is_some() {
        Action::TransmitAndPut
    } else {
        Action::Transmit
    };

    let mut head = Vec::with_capacity(96);
    write!(
        &mut head,
        "a={},f={},t={},s={},v={},i={}",
        action.key() as char,
        t.format as u32,
        t.medium.key() as char,
        t.width,
        t.height,
        t.id
    )?;
    if let Some(p) = t.placement {
        write!(
            &mut head,
            ",p={},X={},Y={},z={},C=1",
            p.id, p.x_off, p.y_off, p.z
        )?;
    }
    // q=2 suppresses replies so they cannot corrupt the parsed input stream.
    head.extend_from_slice(b",q=2");

    if b64.len() <= CHUNK {
        out.write_all(APC_START)?;
        out.write_all(&head)?;
        out.write_all(b";")?;
        out.write_all(&b64)?;
        out.write_all(APC_END)?;
        return Ok(());
    }

    // Full key set on the first chunk; `m=1` on chunks with a successor,
    // `m=0` on the last.
    let mut first = true;
    let mut rest = &b64[..];
    while !rest.is_empty() {
        let n = rest.len().min(CHUNK);
        let (this, next) = rest.split_at(n);
        let more = if next.is_empty() { b'0' } else { b'1' };

        out.write_all(APC_START)?;
        if first {
            out.write_all(&head)?;
            write!(out, ",m={}", more as char)?;
            first = false;
        } else {
            write!(out, "m={},q=2", more as char)?;
        }
        out.write_all(b";")?;
        out.write_all(this)?;
        out.write_all(APC_END)?;
        rest = next;
    }
    Ok(())
}

/// Place an already-transmitted image again (cheap: no payload).
pub fn put<W: Write>(out: &mut W, id: u32, p: Placement) -> std::io::Result<()> {
    out.write_all(APC_START)?;
    write!(
        out,
        "a=p,i={},p={},X={},Y={},z={},C=1,q=2",
        id, p.id, p.x_off, p.y_off, p.z
    )?;
    out.write_all(b";")?;
    out.write_all(APC_END)
}

/// Drop a placement but keep the image data resident.
pub fn delete_placement<W: Write>(out: &mut W, id: u32, placement: u32) -> std::io::Result<()> {
    out.write_all(APC_START)?;
    write!(out, "a=d,d=i,i={},p={},q=2", id, placement)?;
    out.write_all(b";")?;
    out.write_all(APC_END)
}

/// Drop placements and free the image data. Uppercase delete verbs free data;
/// lowercase keep it.
pub fn delete_image<W: Write>(out: &mut W, id: u32) -> std::io::Result<()> {
    out.write_all(APC_START)?;
    write!(out, "a=d,d=I,i={},q=2", id)?;
    out.write_all(b";")?;
    out.write_all(APC_END)
}

/// Move the cursor to a 0-based cell. CUP is 1-based.
pub fn cursor_to<W: Write>(out: &mut W, col: u16, row: u16) -> std::io::Result<()> {
    write!(out, "\x1b[{};{}H", row as u32 + 1, col as u32 + 1)
}

// --- base64 ---------------------------------------------------------------
// Standard alphabet with padding, as the protocol requires.

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len().div_ceil(3) * 4);
    base64_into(src, &mut out);
    out
}

pub fn base64_into(src: &[u8], out: &mut Vec<u8>) {
    let (chunks, remainder) = src.as_chunks::<3>();
    for c in chunks {
        let n = ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32;
        out.push(B64[(n >> 18) as usize & 63]);
        out.push(B64[(n >> 12) as usize & 63]);
        out.push(B64[(n >> 6) as usize & 63]);
        out.push(B64[n as usize & 63]);
    }
    match remainder {
        [a] => {
            let n = (*a as u32) << 16;
            out.push(B64[(n >> 18) as usize & 63]);
            out.push(B64[(n >> 12) as usize & 63]);
            out.push(b'=');
            out.push(b'=');
        }
        [a, b] => {
            let n = ((*a as u32) << 16) | ((*b as u32) << 8);
            out.push(B64[(n >> 18) as usize & 63]);
            out.push(B64[(n >> 12) as usize & 63]);
            out.push(B64[(n >> 6) as usize & 63]);
            out.push(b'=');
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        assert_eq!(base64(b""), b"");
        assert_eq!(base64(b"f"), b"Zg==");
        assert_eq!(base64(b"fo"), b"Zm8=");
        assert_eq!(base64(b"foo"), b"Zm9v");
        assert_eq!(base64(b"foob"), b"Zm9vYg==");
        assert_eq!(base64(b"fooba"), b"Zm9vYmE=");
        assert_eq!(base64(b"foobar"), b"Zm9vYmFy");
    }

    #[test]
    fn shm_transmit_is_single_unchunked_sequence() {
        let mut o = Vec::new();
        transmit(
            &mut o,
            &Transmit {
                id: 7,
                format: Format::Rgba,
                medium: Medium::SharedMem,
                width: 800,
                height: 600,
                payload: b"/kp1234",
                placement: Some(Placement {
                    id: 1,
                    x_off: 3,
                    y_off: 9,
                    z: 2,
                }),
            },
        )
        .unwrap();
        let s = String::from_utf8(o).unwrap();
        assert!(s.starts_with("\x1b_Ga=T,f=32,t=s,s=800,v=600,i=7,p=1,X=3,Y=9,z=2,C=1,q=2;"));
        assert!(s.ends_with("\x1b\\"));
        assert_eq!(s.matches("\x1b_G").count(), 1, "shm name must never chunk");
    }

    #[test]
    fn oversized_direct_payload_chunks_correctly() {
        let mut o = Vec::new();
        let payload = vec![0u8; 12_000];
        transmit(
            &mut o,
            &Transmit {
                id: 1,
                format: Format::Rgba,
                medium: Medium::Direct,
                width: 100,
                height: 30,
                payload: &payload,
                placement: None,
            },
        )
        .unwrap();
        let s = String::from_utf8(o).unwrap();
        assert_eq!(s.matches("\x1b_G").count(), 4);
        assert_eq!(s.matches("m=1").count(), 3);
        assert_eq!(s.matches("m=0").count(), 1);
        assert_eq!(s.matches("f=32").count(), 1);
    }
}
