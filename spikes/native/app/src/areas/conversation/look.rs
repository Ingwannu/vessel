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
//! - Choice: GOAT's values per scheme, named as GOAT names them
//!   (`--message-surface`, `--secondary-label`), one rename from the
//!   kit's theme when the lead adds them there.
//! - Differences from the page's DESIGN.md (the references win): the
//!   page's prompt is a mono block on a card, left-aligned; GOAT's is a
//!   bubble on the right, 70% of the lane at most, radius 16. The page's
//!   column is 760 wide; GOAT's chat lane is 672.

use vessel_app_kit::ui::{Hsla, rgb, rgba};
use vessel_app_kit::{Scheme, Tokens};

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

/// GOAT's colours the kit's theme does not hold.
#[derive(Debug, Clone, Copy)]
pub(super) struct Look {
    /// `--message-surface`: a person's message.
    pub(super) message: Hsla,
    /// `--secondary-label`: the work log's words.
    pub(super) secondary: Hsla,
    /// `--border`: a rule.
    pub(super) border: Hsla,
    /// `--code-background`: a tool's input and output.
    pub(super) code: Hsla,
    /// Lines added and removed in a diff (Paseo's diff colours, which
    /// GOAT's palette leaves to Tailwind's).
    pub(super) added: Hsla,
    pub(super) removed: Hsla,
}

impl Look {
    pub(super) fn of(tokens: &Tokens) -> Self {
        match tokens.scheme {
            Scheme::Light => Self {
                message: rgb(0x00f3_f3f3).into(),
                secondary: rgb(0x005d_5d5d).into(),
                border: rgb(0x00ec_ecec).into(),
                code: rgb(0x00f9_f9f9).into(),
                added: rgb(0x0015_803d).into(),
                removed: rgb(0x00b9_1c1c).into(),
            },
            Scheme::Dark => Self {
                message: rgba(0xffff_ff14).into(),
                secondary: rgb(0x00af_afaf).into(),
                border: rgba(0xffff_ff1a).into(),
                code: rgba(0x0000_0059).into(),
                added: rgb(0x004a_de80).into(),
                removed: rgb(0x00ef_4444).into(),
            },
        }
    }
}
