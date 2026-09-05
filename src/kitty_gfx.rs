//! Present a native-resolution RGBA frame through the existing kitty stack.
//! Shared memory (`t=s`) when the probe succeeds; base64 only as fallback.

use crate::kitty::encoder::{self, Format, Placement, Transmit};
use crate::kitty::transport::Transport;
use std::io::{self, Write};

pub fn available() -> bool {
    if std::env::var_os("TUICRAFT_ASCII").is_some() {
        return false;
    }
    std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var("TERM").unwrap_or_default().contains("kitty")
}

pub struct Presenter {
    transport: Transport,
}

impl Presenter {
    pub fn new() -> Self {
        Self {
            transport: Transport::detect(),
        }
    }

    pub fn present(&mut self, out: &mut impl Write, rgba: &[u8], w: u32, h: u32) -> io::Result<()> {
        let payload = self.transport.publish(rgba)?;
        encoder::transmit(
            out,
            &Transmit {
                id: 1,
                format: Format::Rgba,
                medium: payload.medium,
                width: w,
                height: h,
                payload: payload.bytes,
                placement: Some(Placement {
                    id: 1,
                    x_off: 0,
                    y_off: 0,
                    z: 0,
                }),
            },
        )?;
        self.transport.end_frame();
        Ok(())
    }
}

pub fn delete_all(out: &mut impl Write) {
    let _ = out.write_all(b"\x1b_Ga=d,d=A,q=2\x1b\\");
}
