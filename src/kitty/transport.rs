//! Payload delivery over base64 escape sequences or POSIX shared memory.
//!
//! Copied from termin8-command-palette (`src/kitty/transport.rs`). Shared
//! memory (`t=s`) is the live path; base64 through the pty is the fallback.
//!
//! Shared memory sends kitty a short object name instead of the pixels. macOS
//! caps names at PSHMNAMLEN (31 bytes incl. leading '/'); kitty unlinks objects
//! after reading them, and we unlink any that age out after RECLAIM_LAG frames.

use super::encoder::Medium;
use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicU32, Ordering};

/// Frames a shm name stays un-unlinked before we reclaim it. Unlinking before
/// kitty has mmap'd the object silently drops the layer, so this is deliberately
/// generous (~1.5 s) rather than tight.
const RECLAIM_LAG: usize = 90;

/// Process-global name counter so concurrent `Transport`s never generate the
/// same name and fail the `O_EXCL` create.
static COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct Transport {
    kind: Medium,
    pid: u32,
    /// (name, frame it was published on)
    pending: VecDeque<(std::ffi::CString, u64)>,
    frame: u64,
    /// Scratch reused across frames so the Direct path never reallocates.
    scratch: Vec<u8>,
}

/// A payload ready to hand to `encoder::transmit`, plus the medium to declare.
pub struct Payload<'a> {
    pub medium: Medium,
    pub bytes: &'a [u8],
}

impl Transport {
    /// Direct-transfer transport for use inside another terminal emulator; the
    /// host reads the APC byte stream itself.
    pub fn direct() -> Transport {
        Transport {
            kind: Medium::Direct,
            pid: std::process::id(),
            pending: VecDeque::new(),
            frame: 0,
            scratch: Vec::new(),
        }
    }

    /// Probe shared-memory support once at startup; degrade to base64 on any
    /// failure.
    pub fn detect() -> Transport {
        let mut t = Transport {
            kind: Medium::SharedMem,
            pid: std::process::id(),
            pending: VecDeque::new(),
            frame: 0,
            scratch: Vec::new(),
        };
        match t.try_shm(&[0u8; 4]) {
            Ok(name) => {
                // Probe succeeded; unlink it, nobody will read it.
                unsafe { libc::shm_unlink(name.as_ptr()) };
                t.pending.clear();
            }
            Err(_) => t.kind = Medium::Direct,
        }
        t
    }

    pub fn medium(&self) -> Medium {
        self.kind
    }

    /// Publish `pixels` and return the payload bytes to embed in the escape.
    /// The returned slice borrows `self`, so the escape must be written before
    /// the next `publish`.
    pub fn publish(&mut self, pixels: &[u8]) -> io::Result<Payload<'_>> {
        match self.kind {
            Medium::SharedMem => {
                let name = self.try_shm(pixels)?;
                self.pending.push_back((name, self.frame));
                let last = &self.pending.back().unwrap().0;
                Ok(Payload {
                    medium: Medium::SharedMem,
                    // Name without trailing NUL, base64'd by the encoder;
                    // leading '/' is part of the name.
                    bytes: last.as_bytes(),
                })
            }
            _ => {
                self.scratch.clear();
                self.scratch.extend_from_slice(pixels);
                Ok(Payload {
                    medium: Medium::Direct,
                    bytes: &self.scratch,
                })
            }
        }
    }

    /// Call once per frame after the frame has been flushed to the tty.
    pub fn end_frame(&mut self) {
        self.frame += 1;
        while let Some((_, f)) = self.pending.front() {
            if self.frame.saturating_sub(*f) < RECLAIM_LAG as u64 {
                break;
            }
            let (name, _) = self.pending.pop_front().unwrap();
            // Best effort; ENOENT means kitty already unlinked it.
            unsafe { libc::shm_unlink(name.as_ptr()) };
        }
    }

    fn try_shm(&mut self, pixels: &[u8]) -> io::Result<std::ffi::CString> {
        // Short by construction: "/kp" + pid + counter stays under macOS's
        // 31-byte PSHMNAMLEN.
        //
        // A few attempts: EEXIST means a stale name from a crashed run, and
        // bumping the counter walks past it.
        let mut cname = std::ffi::CString::default();
        let mut fd = -1;
        let mut last_err = io::Error::from(io::ErrorKind::AlreadyExists);
        for _ in 0..8 {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
            cname = std::ffi::CString::new(format!("/kp{:x}{:x}", self.pid & 0xffff, n)).unwrap();
            // SAFETY: cname is NUL-terminated and outlives the call.
            fd = unsafe {
                libc::shm_open(
                    cname.as_ptr(),
                    libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
                    0o600 as libc::c_uint,
                )
            };
            if fd >= 0 {
                break;
            }
            last_err = io::Error::last_os_error();
            if last_err.kind() != io::ErrorKind::AlreadyExists {
                return Err(last_err);
            }
        }
        if fd < 0 {
            return Err(last_err);
        }

        let len = pixels.len();
        // macOS allows ftruncate on a shm object only once, right after creation.
        if unsafe { libc::ftruncate(fd, len as libc::off_t) } != 0 {
            let e = io::Error::last_os_error();
            unsafe {
                libc::close(fd);
                libc::shm_unlink(cname.as_ptr());
            }
            return Err(e);
        }

        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        // The fd is not needed once mapped.
        unsafe { libc::close(fd) };
        if map == libc::MAP_FAILED {
            let e = io::Error::last_os_error();
            unsafe { libc::shm_unlink(cname.as_ptr()) };
            return Err(e);
        }

        // SAFETY: `map` is a valid writable mapping of exactly `len` bytes and
        // does not overlap `pixels` (fresh anonymous shared page).
        unsafe {
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), map as *mut u8, len);
            libc::munmap(map, len);
        }

        Ok(cname)
    }
}

impl Drop for Transport {
    fn drop(&mut self) {
        for (name, _) in self.pending.drain(..) {
            unsafe { libc::shm_unlink(name.as_ptr()) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_transport_is_always_direct() {
        let mut t = Transport::direct();
        assert_eq!(t.medium(), Medium::Direct);
        let payload = t.publish(&[1, 2, 3, 4]).unwrap();
        assert_eq!(payload.medium, Medium::Direct);
        assert_eq!(payload.bytes, &[1, 2, 3, 4]);
    }

    #[test]
    fn shm_names_fit_macos_limit() {
        for n in [1u32, 0xffff, 0xffff_ffff] {
            let name = format!("/kp{:x}{:x}", 0xffffu32, n);
            assert!(name.len() < 31, "{name} too long for PSHMNAMLEN");
        }
    }

    #[test]
    fn concurrent_transports_do_not_collide_on_names() {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(|| {
                    let mut t = Transport::detect();
                    if t.medium() != Medium::SharedMem {
                        return true;
                    }
                    (0..25).all(|_| t.publish(&[7u8; 512]).is_ok())
                })
            })
            .collect();
        for h in handles {
            assert!(h.join().unwrap(), "shm name collision under concurrency");
        }
    }

    #[test]
    fn reclaims_only_after_lag() {
        let mut t = Transport::detect();
        if t.medium() != Medium::SharedMem {
            return;
        }
        t.publish(&[1u8; 64]).unwrap();
        assert_eq!(t.pending.len(), 1);
        for _ in 0..RECLAIM_LAG - 1 {
            t.end_frame();
        }
        assert_eq!(t.pending.len(), 1, "reclaimed too eagerly");
        t.end_frame();
        assert_eq!(t.pending.len(), 0);
    }
}
