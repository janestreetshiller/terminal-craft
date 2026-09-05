//! Optional UI skin. Theme state never enters world or player saves.
#![allow(clippy::too_many_arguments)]

use crate::render::{blit_text_px, fill_rect, stroke_rect};
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) const GOLD: (u8, u8, u8) = (220, 167, 58);
pub(crate) const CREAM: (u8, u8, u8) = (255, 232, 173);
pub(crate) const INK: (u8, u8, u8) = (39, 25, 15);
pub(crate) const MUTED: (u8, u8, u8) = (187, 164, 116);
static GILDED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Theme {
    Classic,
    Gilded,
}

impl Theme {
    pub(crate) fn parse(name: &str) -> Result<Self, &'static str> {
        match name {
            "classic" => Ok(Self::Classic),
            "gilded" => Ok(Self::Gilded),
            _ => Err("TERMINAL_CRAFT_UI must be classic or gilded"),
        }
    }
}

pub(crate) fn set(theme: Theme) {
    GILDED.store(theme == Theme::Gilded, Ordering::Relaxed);
}
pub(crate) fn gilded() -> bool {
    GILDED.load(Ordering::Relaxed)
}
pub(crate) fn toggle() {
    GILDED.fetch_xor(true, Ordering::Relaxed);
}
pub(crate) fn name() -> &'static str {
    if gilded() {
        "GILDED"
    } else {
        "CLASSIC"
    }
}

/// Original pixel geometry; no imported texture pack or external assets.
pub(crate) fn frame(
    p: &mut [u8],
    w: i32,
    h: i32,
    x: i32,
    y: i32,
    rw: i32,
    rh: i32,
    selected: bool,
    disabled: bool,
) {
    if rw < 10 || rh < 10 {
        return;
    }
    let edge = if disabled {
        (101, 82, 46)
    } else if selected {
        CREAM
    } else {
        GOLD
    };
    let face = if disabled {
        (47, 41, 29)
    } else if selected {
        (113, 74, 23)
    } else {
        (72, 47, 23)
    };
    fill_rect(p, w, h, x, y, rw, rh, INK);
    stroke_rect(p, w, h, x + 1, y + 1, rw - 2, rh - 2, 2, edge);
    fill_rect(p, w, h, x + 4, y + 4, rw - 8, rh - 8, face);
    fill_rect(
        p,
        w,
        h,
        x + 4,
        y + 4,
        rw - 8,
        2,
        if disabled {
            (74, 64, 44)
        } else {
            (163, 116, 40)
        },
    );
    fill_rect(p, w, h, x + 4, y + rh - 6, rw - 8, 2, (43, 28, 15));
    // Forged corner caps, inset highlights and dark rivets.
    if rw >= 30 && rh >= 24 {
        for cx in [x + 3, x + rw - 10] {
            fill_rect(p, w, h, cx, y + 3, 7, rh - 6, edge);
            fill_rect(
                p,
                w,
                h,
                cx + 2,
                y + 6,
                3,
                rh - 12,
                if disabled {
                    (78, 64, 40)
                } else {
                    (158, 107, 33)
                },
            );
            for cy in [y + 6, y + rh - 9] {
                fill_rect(p, w, h, cx + 2, cy, 3, 3, INK);
                fill_rect(p, w, h, cx + 2, cy, 1, 1, CREAM);
            }
        }
    }
    if selected && !disabled {
        fill_rect(p, w, h, x + rw / 2 - 5, y, 10, 2, CREAM);
        fill_rect(p, w, h, x + rw / 2 - 3, y + 2, 6, 2, CREAM);
    }
}

pub(crate) fn panel(p: &mut [u8], w: i32, h: i32, x: i32, y: i32, rw: i32, rh: i32) {
    frame(p, w, h, x, y, rw, rh, false, false);
    fill_rect(p, w, h, x + 12, y + 8, rw - 24, rh - 16, (45, 32, 22));
    // Restrained grain lives only inside chrome, never in world pixels.
    for yy in ((y + 12)..(y + rh - 10)).step_by(8) {
        fill_rect(p, w, h, x + 12, yy, rw - 24, 1, (50, 36, 24));
    }
}

pub(crate) fn slot(
    p: &mut [u8],
    w: i32,
    h: i32,
    x: i32,
    y: i32,
    size: i32,
    selected: bool,
    deny: bool,
) {
    fill_rect(p, w, h, x, y, size, size, INK);
    stroke_rect(
        p,
        w,
        h,
        x + 1,
        y + 1,
        size - 2,
        size - 2,
        2,
        if deny {
            (238, 101, 71)
        } else if selected {
            CREAM
        } else {
            GOLD
        },
    );
    fill_rect(
        p,
        w,
        h,
        x + 4,
        y + 4,
        size - 8,
        size - 8,
        if selected {
            (245, 219, 156)
        } else {
            (198, 162, 96)
        },
    );
    fill_rect(p, w, h, x + 4, y + 4, size - 8, 2, CREAM);
    fill_rect(p, w, h, x + 4, y + size - 6, size - 8, 2, (139, 96, 41));
    if selected {
        stroke_rect(p, w, h, x - 2, y - 2, size + 4, size + 4, 1, CREAM);
    }
}

pub(crate) fn text(
    p: &mut [u8],
    w: i32,
    h: i32,
    x: i32,
    y: i32,
    label: &str,
    color: (u8, u8, u8),
    scale: f32,
) {
    let d = scale.round().max(1.0) as i32;
    blit_text_px(p, w, h, x + d, y + d, label, INK, scale);
    blit_text_px(p, w, h, x, y, label, color, scale);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_names_are_explicit_and_unknown_names_fail() {
        assert_eq!(Theme::parse("classic"), Ok(Theme::Classic));
        assert_eq!(Theme::parse("gilded"), Ok(Theme::Gilded));
        assert!(Theme::parse("gold-ish").is_err());
    }

    #[test]
    fn gilded_frame_is_clipped_and_states_are_distinct() {
        let mut normal = vec![0; 120 * 60 * 4];
        frame(&mut normal, 120, 60, 10, 10, 100, 40, false, false);
        assert!(normal.iter().any(|&v| v != 0));
        assert_eq!(&normal[..120 * 4], &vec![0; 120 * 4]);
        let mut selected = vec![0; 120 * 60 * 4];
        frame(&mut selected, 120, 60, 10, 10, 100, 40, true, false);
        let mut disabled = vec![0; 120 * 60 * 4];
        frame(&mut disabled, 120, 60, 10, 10, 100, 40, false, true);
        assert_ne!(normal, selected);
        assert_ne!(normal, disabled);
        let mut tiny = vec![0; 16 * 16 * 4];
        frame(&mut tiny, 16, 16, -10, -10, 30, 30, true, false);
        frame(&mut tiny, 16, 16, 0, 0, 0, 0, false, false);
    }
}
