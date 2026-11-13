//! GOAT's inline status roles, from `SideChatSurface`'s text/dot classes,
//! `index.css` and `themePalettes.ts`. Text and dot are separate semantics;
//! a badge tint or an alert-icon blue cannot stand in for either.

use gpui::{Hsla, rgb};

use super::Scheme;

/// A readable foreground paired with its solid status dot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DotSignal {
    /// The label's foreground, not a fill or an icon accent.
    pub foreground: Hsla,
    /// The dot: distinct from the text for attention and error.
    pub dot: Hsla,
}

/// The four inline roles, with no capsule fill or border.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InlineStatus {
    /// `text-info-foreground` and `bg-info`.
    pub active: DotSignal,
    /// `text-warning-foreground` and `bg-warning`.
    pub attention: DotSignal,
    /// `text-muted-foreground` and `bg-muted-foreground/60`.
    pub idle: DotSignal,
    /// `text-destructive-foreground` and `bg-destructive`.
    pub error: DotSignal,
}

impl InlineStatus {
    /// Resolved GOAT default semantic values in each appearance. The
    /// existing Paseo capsule palette has no equivalent text/dot pairs;
    /// keeping these roles here avoids per-screen colour choices.
    #[must_use]
    pub fn of(scheme: Scheme) -> Self {
        let (information, muted, warning_text, error_text, error_dot) = match scheme {
            Scheme::Light => (
                0x0001_69cc,
                0x005d_5d5d,
                0x00bb_4d00,
                0x00c1_0007,
                0x00fb_2c36,
            ),
            Scheme::Dark => (
                0x0033_9cff,
                0x00af_afaf,
                0x00ff_b900,
                0x00ff_6467,
                0x00fb_414a,
            ),
        };
        let idle_foreground: Hsla = rgb(muted).into();
        let mut idle_dot = idle_foreground;
        idle_dot.a = 0.6;
        Self {
            active: DotSignal {
                foreground: rgb(information).into(),
                dot: rgb(information).into(),
            },
            attention: DotSignal {
                foreground: rgb(warning_text).into(),
                dot: rgb(0x00fe_9a00).into(),
            },
            idle: DotSignal {
                foreground: idle_foreground,
                dot: idle_dot,
            },
            error: DotSignal {
                foreground: rgb(error_text).into(),
                dot: rgb(error_dot).into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_resolved_reference_palettes_keep_text_and_dot_distinct() {
        for (scheme, info, muted, warning, error, dot) in [
            (
                Scheme::Light,
                0x0001_69cc,
                0x005d_5d5d,
                0x00bb_4d00,
                0x00c1_0007,
                0x00fb_2c36,
            ),
            (
                Scheme::Dark,
                0x0033_9cff,
                0x00af_afaf,
                0x00ff_b900,
                0x00ff_6467,
                0x00fb_414a,
            ),
        ] {
            let palette = InlineStatus::of(scheme);
            assert_eq!(palette.active.foreground, rgb(info).into());
            assert_eq!(palette.active.dot, rgb(info).into());
            assert_eq!(palette.attention.foreground, rgb(warning).into());
            assert_eq!(palette.attention.dot, rgb(0x00fe_9a00).into());
            assert_eq!(palette.idle.foreground, rgb(muted).into());
            let mut translucent: Hsla = rgb(muted).into();
            translucent.a = 0.6;
            assert_eq!(palette.idle.dot, translucent);
            assert_eq!(palette.error.foreground, rgb(error).into());
            assert_eq!(palette.error.dot, rgb(dot).into());
        }
    }
}
