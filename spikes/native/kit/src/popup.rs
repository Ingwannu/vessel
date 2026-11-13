//! The popup surface every floating thing shares (menus, dialogs, and
//! the lead's sheets and tooltips): DESIGN.md section 4.6, the
//! reference's popup recipe, so they cannot drift apart.
//!
//! The design's popup tokens (`--popup-edge`, `--popup-shadow`,
//! `--overlay`) are the ink or black at a given strength; they are
//! derived here from the scheme's tokens rather than added to every
//! scheme's table, so the theme file stays the lead's.

use gpui::{BoxShadow, Hsla, black, point, px};

use crate::theme::{Scheme, Tokens};

/// The popup's fill (`--popup-bg`: white; `#2E2E2E` in dark).
#[must_use]
pub const fn fill(tokens: &Tokens) -> Hsla {
    tokens.bg_elevated
}

/// The popup's edge (`--popup-edge`): the ink at 2%.
#[must_use]
pub const fn edge(tokens: &Tokens) -> Hsla {
    Hsla {
        a: 0.02,
        ..tokens.text_primary
    }
}

/// The popup's shadow (`--popup-shadow`): `0 4px 12px -4px` of the ink at
/// 5%, black at 14% in dark.
#[must_use]
pub fn shadow(tokens: &Tokens) -> Vec<BoxShadow> {
    let color = match tokens.scheme {
        Scheme::Light => Hsla {
            a: 0.05,
            ..tokens.text_primary
        },
        Scheme::Dark => Hsla { a: 0.14, ..black() },
    };
    vec![BoxShadow {
        color,
        offset: point(px(0.0), px(4.0)),
        blur_radius: px(12.0),
        spread_radius: px(-4.0),
        inset: false,
    }]
}

/// A dialog's scrim (`--overlay`): the ink at 45% light, the page's own
/// black at 85% dark. No blur behind it.
#[must_use]
pub const fn scrim(tokens: &Tokens) -> Hsla {
    match tokens.scheme {
        Scheme::Light => Hsla {
            a: 0.45,
            ..tokens.text_primary
        },
        Scheme::Dark => Hsla {
            a: 0.85,
            ..tokens.bg_shell
        },
    }
}

/// A row's height in a popup: the reference's 32 pixels, 44 on a narrow
/// (touch) window (ADR 0036).
#[must_use]
pub const fn row_height(narrow: bool) -> f32 {
    if narrow { 44.0 } else { 32.0 }
}
