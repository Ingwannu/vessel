//! Generic contributed paint, with no contribution identity, parsing or I/O.
//! Source: Paseo plugins/themes/index.ts:63–126. The app maps validated
//! contribution colours here, then reapplies its local typography/syntax.
use crate::theme::{Neutrals, Scheme, Tokens};
use gpui::Hsla;

/// Semantic input colours for one concrete custom palette. All alpha is
/// preserved; the boundary owner has already resolved optional accent.
#[derive(Debug, Clone, Copy)]
pub struct ContributedColors {
    /// Main background.
    pub background: Hsla,
    /// Main text.
    pub foreground: Hsla,
    /// Raised card background.
    pub raised: Hsla,
    /// Input/control background.
    pub control: Hsla,
    /// Divider and stronger control edge.
    pub border: Hsla,
    /// Already resolved accent, including its foreground fallback.
    pub accent: Hsla,
    /// Secondary text.
    pub muted_foreground: Hsla,
    /// Ring and extra-muted step.
    pub ring: Hsla,
}

impl Tokens {
    /// Map a validated concrete paint using the same neutral-to-token builder
    /// as named tints. Built-in status/destructive signals follow its scheme.
    #[must_use]
    pub fn contributed(scheme: Scheme, colors: ContributedColors) -> Self {
        let base = Self::of(scheme);
        let neutral = Neutrals {
            surface0: colors.background,
            surface1: colors.raised,
            surface2: colors.control,
            surface3: colors.border,
            surface4: colors.ring,
            surface_sidebar: match scheme {
                Scheme::Light => colors.control,
                Scheme::Dark => colors.background,
            },
            foreground: colors.foreground,
            foreground_muted: colors.muted_foreground,
            foreground_extra_muted: colors.ring,
            sheet_handle: base.neutral.sheet_handle,
            border: colors.border,
            border_accent: colors.border,
            interaction_highlight: base.neutral.interaction_highlight,
            destructive: base.neutral.destructive,
            destructive_foreground: match scheme {
                Scheme::Light => colors.background,
                Scheme::Dark => base.neutral.destructive_foreground,
            },
            accent: colors.accent,
            accent_foreground: colors.background,
        };
        super::mapped(base, neutral, colors.ring)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_aliases_preserve_alpha_and_shared_status_by_scheme() {
        let colors = ContributedColors {
            background: gpui::rgba(0x1020_30a0).into(),
            foreground: gpui::rgb(0x00ee_ddcc).into(),
            raised: gpui::rgb(0x0020_3040).into(),
            control: gpui::rgb(0x0030_4050).into(),
            border: gpui::rgb(0x0040_5060).into(),
            accent: gpui::rgb(0x0050_6070).into(),
            muted_foreground: gpui::rgb(0x0060_7080).into(),
            ring: gpui::rgba(0x7080_90c0).into(),
        };
        for scheme in [Scheme::Light, Scheme::Dark] {
            let built_in = Tokens::of(scheme);
            let custom = Tokens::contributed(scheme, colors);
            assert_eq!(custom.bg_base, colors.background);
            assert_eq!(custom.bg_surface, colors.raised);
            assert_eq!(
                custom.bg_elevated,
                match scheme {
                    Scheme::Light => colors.background,
                    Scheme::Dark => colors.control,
                }
            );
            assert_eq!(custom.text_primary, colors.foreground);
            assert_eq!(custom.ring, colors.ring);
            assert_eq!(custom.neutral.accent_foreground, colors.background);
            assert_eq!(custom.neutral.destructive, built_in.neutral.destructive);
            assert_eq!(custom.st_needs_you, built_in.st_needs_you);
            assert_eq!(custom.scheme, scheme);
            assert_eq!(
                custom.bg_shell,
                match scheme {
                    Scheme::Light => colors.control,
                    Scheme::Dark => colors.background,
                }
            );
        }
    }
}
