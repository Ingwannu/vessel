//! Paseo's semantic status signals (`styles/theme.ts`), separate from
//! the board's lane colours and its brighter status dots. A badge's tint
//! always belongs to the same signal as its text.

use super::Scheme;
use gpui::{Hsla, rgb, rgba};

/// A semantic signal and its translucent badge fill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Signal {
    /// The text colour.
    pub foreground: Hsla,
    /// The same hue at the reference's theme-specific alpha.
    pub tint: Hsla,
}

/// Paseo's status signals, in one theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Status {
    /// Success, such as access granted or a finished run.
    pub success: Signal,
    /// A state that needs attention.
    pub warning: Signal,
    /// An error or refused operation.
    pub error: Signal,
}

impl Status {
    /// The reference's values: 12% tint in light, 16% in dark.
    #[must_use]
    pub fn of(scheme: Scheme) -> Self {
        match scheme {
            Scheme::Light => Self {
                success: Signal {
                    foreground: rgb(0x003e_704a).into(),
                    tint: rgba(0x3e70_4a1f).into(),
                },
                warning: Signal {
                    foreground: rgb(0x007b_5d39).into(),
                    tint: rgba(0x7b5d_391f).into(),
                },
                error: Signal {
                    foreground: rgb(0x009d_433b).into(),
                    tint: rgba(0x9d43_3b1f).into(),
                },
            },
            Scheme::Dark => Self {
                success: Signal {
                    foreground: rgb(0x006c_b17b).into(),
                    tint: rgba(0x6cb1_7b29).into(),
                },
                warning: Signal {
                    foreground: rgb(0x00c0_9664).into(),
                    tint: rgba(0xc096_6429).into(),
                },
                error: Signal {
                    foreground: rgb(0x00d8_847b).into(),
                    tint: rgba(0xd884_7b29).into(),
                },
            },
        }
    }
}
