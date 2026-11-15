//! One quota bar for account cards and quiet sidebar summaries, after Paseo meter.tsx.
//!
//! The caller owns units, report freshness and accessibility text. This renderer
//! receives only a bounded display share and tokens; it cannot infer account usage.
use crate::{Tokens, theme::spacing};
use gpui::{Div, Hsla, div, prelude::*, px, relative};

const FULL_PERCENT: u8 = 100;

/// An integer display share is bounded before entering layout arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Share(u8);
impl Share {
    /// Values beyond a full window fill the track and never escape its bounds.
    #[must_use]
    pub const fn percent(value: u8) -> Self {
        Self(if value > FULL_PERCENT {
            FULL_PERCENT
        } else {
            value
        })
    }
    #[expect(
        clippy::float_arithmetic,
        reason = "A bounded quota share is converted to relative layout geometry, never accounting"
    )]
    fn fraction(self) -> f32 {
        f32::from(self.0) / f32::from(FULL_PERCENT)
    }
}

/// Source-provided tone or the view's fallback; the kit does not choose thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Ordinary muted foreground.
    Default,
    /// Source says this quota is healthy.
    Ok,
    /// Source or view warns that the quota is running low.
    Warning,
    /// Source or view reports danger.
    Danger,
}

/// Quiet summaries deliberately remove hue, without changing their share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    /// Account cards use semantic quota colour.
    Normal(Tone),
    /// Sidebar summary bars use the fourth surface.
    Quiet,
}

fn fill(appearance: Appearance, tokens: &Tokens) -> Hsla {
    match appearance {
        Appearance::Quiet => tokens.neutral.surface4,
        Appearance::Normal(tone) => match tone {
            Tone::Default => tokens.neutral.foreground_muted,
            Tone::Ok => tokens.status().success.foreground,
            Tone::Warning => tokens.status().warning.foreground,
            Tone::Danger => tokens.status().error.foreground,
        },
    }
}

/// Paseo's six-point track, full-radius fill and independently sized caller width.
#[must_use]
pub fn bar(share: Share, appearance: Appearance, tokens: &Tokens) -> Div {
    div()
        .w_full()
        .min_w_0()
        .h(px(spacing::S1_5))
        .rounded_full()
        .overflow_hidden()
        .bg(tokens.neutral.surface3)
        .child(
            div()
                .h_full()
                .w(relative(share.fraction()))
                .rounded_full()
                .bg(fill(appearance, tokens)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scheme;
    #[test]
    fn quiet_summary_uses_surface_while_cards_keep_semantic_tones() {
        for scheme in [Scheme::Dark, Scheme::Light] {
            let tokens = Tokens::of(scheme);
            assert_eq!(fill(Appearance::Quiet, &tokens), tokens.neutral.surface4);
            assert_eq!(
                fill(Appearance::Normal(Tone::Warning), &tokens),
                tokens.status().warning.foreground
            );
            assert_eq!(
                fill(Appearance::Normal(Tone::Danger), &tokens),
                tokens.status().error.foreground
            );
        }
    }
    #[test]
    fn an_excess_share_has_exactly_the_full_tracks_geometry() {
        assert_eq!(
            Share::percent(FULL_PERCENT.saturating_add(1)),
            Share::percent(FULL_PERCENT)
        );
        assert_eq!(Share::percent(0), Share(0));
    }
}
