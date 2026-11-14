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
//!   column is 760 wide; GOAT's chat lane is 672. The app's conversation
//!   sits in the session's one chat column with the composer (Paseo's
//!   820, the session area's `column`), so the two line up: a deliberate
//!   difference from GOAT's 672.

use vessel_app_kit::Tokens;
use vessel_app_kit::ui::Hsla;

/// A person's message: its widest share of the lane (Paseo's: the whole
/// column; GOAT's was 70%).
pub(super) const BUBBLE_SHARE: f32 = 1.0;

/// A person's message: its radius (GOAT's `rounded-2xl`).
pub(super) const BUBBLE_RADIUS: f32 = 16.0;

/// A bubble's corner toward its writer (Paseo's `borderRadius.sm`).
pub(super) const CORNER: f32 = 4.0;

/// The circle beside the agent's reply, and the room between them.
pub(super) const AVATAR: f32 = 24.0;
pub(super) const AVATAR_GAP: f32 = 8.0;

/// Where the reply bubbles start: [`AVATAR`] and [`AVATAR_GAP`], kept
/// equal to their sum by hand (no float arithmetic here). The turn's
/// footer and a failed turn's line start there too.
pub(super) const BUBBLE_COLUMN: f32 = 32.0;

/// The work log's text: 12.5 points on a 1.625 line (GOAT's
/// `--text-work-log`).
pub(super) const WORK_SIZE: f32 = 12.5;
pub(super) const WORK_LINE: f32 = 20.0;

/// The conversation's colours, from the kit's neutral steps (Paseo's
/// scale, which GOAT's chat matches within a step): a person's message
/// on `surface2` (GOAT's `--message-surface`, #f3f3f3 against Paseo's
/// #f4f4f5), the work log's words in `foregroundMuted` (GOAT's
/// `--secondary-label`), rules in `border`. Code and a tool's text are
/// the kit's (`chat_markdown`, `output`), on GOAT's code surface.
#[derive(Debug, Clone, Copy)]
pub(super) struct Look {
    /// A person's message (Paseo's `UserMessage` bubble, `surface3`).
    pub(super) message: Hsla,
    /// The agent's reply bubble (the owner's, slice M): a lighter surface
    /// than the person's, edged with a hairline.
    pub(super) reply: Hsla,
    /// The work log's words.
    pub(super) secondary: Hsla,
    /// A rule.
    pub(super) border: Hsla,
}

impl Look {
    pub(super) const fn of(tokens: &Tokens) -> Self {
        let neutral = &tokens.neutral;
        Self {
            message: neutral.surface3,
            reply: neutral.surface1,
            secondary: neutral.foreground_muted,
            border: neutral.border,
        }
    }
}
