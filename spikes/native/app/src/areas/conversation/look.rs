//! The conversation's look: GOAT's chat (`apps/web/src`, its new design,
//! the default) for the timeline, as the owner's rule after build
//! 0.0.0.0.3 and this track's brief name it. Values and sources are in the
//! reference notes' "Appendix G" and in GOAT's `components/chat/`.
//!
//! [Decision Log]
//! - Purpose: one place for the conversation's measures and the few
//!   colours GOAT's chat draws that the kit's theme does not hold yet (a
//!   person's message surface, the work log's quiet label), so a row
//!   never holds a literal.
//! - Choice: GOAT's measures here; its colours taken from the kit's
//!   neutral steps (`tokens.neutral`), so every colour exists once.
//! - Differences from the page's DESIGN.md (the references win): the
//!   page's prompt is a mono block on a card, left-aligned; GOAT's is a
//!   bubble on the right, 70% of the lane at most, radius 16. The page's
//!   column is 760 wide; GOAT's chat lane is 672.

use vessel_app_kit::Tokens;
use vessel_app_kit::ui::Hsla;

/// The lane's inset each side (GOAT's 0.75rem).
pub(super) const INSET: f32 = 12.0;

/// The lane drawn: GOAT's chat lane, 672 wide at most
/// (`--chat-content-max-width`, 42rem), and `INSET` each side.
pub(super) const LANE_OUTER: f32 = 696.0;

/// A person's message: its widest share of the lane.
pub(super) const BUBBLE_SHARE: f32 = 0.7;

/// A person's message: its radius (GOAT's `rounded-2xl`).
pub(super) const BUBBLE_RADIUS: f32 = 16.0;

/// The work log's text: 12.5 points on a 1.625 line (GOAT's
/// `--text-work-log`).
pub(super) const WORK_SIZE: f32 = 12.5;
pub(super) const WORK_LINE: f32 = 20.0;

/// The conversation's colours, from the kit's neutral steps (Paseo's
/// scale, which GOAT's chat matches within a step): a person's message
/// on `surface2` (GOAT's `--message-surface`, #f3f3f3 against Paseo's
/// #f4f4f5), the work log's words in `foregroundMuted` (GOAT's
/// `--secondary-label`), rules in `border`, code on `surface1`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Look {
    /// A person's message.
    pub(super) message: Hsla,
    /// The work log's words.
    pub(super) secondary: Hsla,
    /// A rule.
    pub(super) border: Hsla,
    /// A tool's input and output.
    pub(super) code: Hsla,
    /// Lines added and removed in a diff.
    pub(super) added: Hsla,
    pub(super) removed: Hsla,
}

impl Look {
    pub(super) const fn of(tokens: &Tokens) -> Self {
        let neutral = &tokens.neutral;
        Self {
            message: neutral.surface2,
            secondary: neutral.foreground_muted,
            border: neutral.border,
            code: neutral.surface1,
            added: tokens.st_ready,
            removed: neutral.destructive,
        }
    }
}
