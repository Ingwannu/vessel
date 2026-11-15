//! The command palette's pieces, as Paseo's command center
//! (`command-center/command-center.tsx`, its `list-metrics.ts`, and
//! `components/ui/shortcut.tsx` for a key's badge): a panel at the top of
//! the window over a dimmed page, its search in a header over a rule, then
//! its groups, each under a muted heading and after a rule, of rows with a
//! glyph, words and the row's keys.
//!
//! [Decision Log]
//! - Purpose: the page's palette (⌘K) in Paseo's look, cloned closely.
//! - Choice: the kit draws the frame, a group's heading, a row, a key's
//!   badges and the empty line; the screen keeps what is listed, which
//!   row is active and what choosing one does. Paseo's values: the panel
//!   640 by 560, at most 92% by 80% of the window, 48 under its top, a
//!   hairline of `border`, 8 round, `surface0`, the large shadow, over
//!   black at 50%; the header 16 across and 12 down, the search 14 with 4
//!   above and below; a group's heading 12 on 16, `foregroundMuted`, 16
//!   across, 8 above and below, after a rule with 8 on each side but the
//!   first; a row 16 across, 6 above and below (12 on a phone), its glyph
//!   16 wide on a 20 line, 12 from its words, the title 14 (on 18 over a
//!   second line, else on 20) in the foreground, the second line 12 on 16
//!   muted, the active row `surface1`; a key's badge 4 by 2 inside, 6
//!   round, `surface3` at half, 12 muted, one per chord 4 apart; the
//!   search's mark `surface3` behind the foreground; the empty line 14
//!   muted, centred, 16 by 24 inside. On a phone it is a sheet from the
//!   bottom over black at 45%, with its handle.
//! - Motion: opt-in persistent `CommandState` adds RNW's 250ms fade and
//!   Gorhom's 60/90% snaps/pan-to-close; the static entry point is retained
//!   for compatibility. Focus restoration remains the host's responsibility.

mod frame;
mod marks;
mod row;
mod state;
#[cfg(test)]
mod tests;
mod text;

pub use frame::{CommandAnimation, CommandHandlers, command_frame, command_frame_animated};

pub use state::{CommandMotion, CommandState};

pub use row::{
    CommandMatches, CommandRow, CommandRowParts, command_row, command_row_matched,
    command_row_parts, key_badges,
};
pub use text::{CommandSubtitle, CommandText};

use gpui::prelude::*;
use gpui::{AnyElement, BoxShadow, Div, SharedString, div, point, px, rgba};

use crate::fonts::{self, Type};
use crate::theme::{Scheme, Tokens, spacing};
use crate::widgets::TypeExt as _;

/// Paseo's panel width.
pub const WIDTH: f32 = 640.0;

/// Paseo's panel height.
const HEIGHT: f32 = 560.0;

/// Between the window's top and the panel (Paseo's `spacing[12]`).
const TOP: f32 = spacing::S12;

/// Around a group's heading and its rule (`COMMAND_CENTER_SECTION_GAP`).
const GAP: f32 = spacing::S2;

/// A row's and a heading's inset across.
const ACROSS: f32 = spacing::S4;

/// Paseo's `theme.shadow.lg`.
#[must_use]
pub fn shadow_lg(tokens: &Tokens) -> Vec<BoxShadow> {
    let (alpha, offset) = match tokens.scheme {
        Scheme::Light => (0x14, 8.0),
        Scheme::Dark => (0x66, 12.0),
    };
    vec![BoxShadow {
        color: rgba(alpha).into(),
        offset: point(px(0.0), px(offset)),
        blur_radius: px(24.0),
        spread_radius: px(0.0),
        inset: false,
    }]
}

/// What the frame holds.
pub struct CommandFrame {
    /// The search field.
    pub search: AnyElement,
    /// The groups and rows, or the empty line.
    pub list: AnyElement,
    /// A phone's width: a sheet from the bottom.
    pub compact: bool,
}

impl std::fmt::Debug for CommandFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandFrame")
            .field("compact", &self.compact)
            .finish_non_exhaustive()
    }
}

/// A key the palette answers before its search does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKey {
    /// The row above.
    Up,
    /// The row below.
    Down,
    /// Run the active row.
    Enter,
    /// Close.
    Escape,
}

/// A group's heading; after a rule unless it is the `first`.
#[must_use]
pub fn command_heading(label: impl Into<SharedString>, first: bool, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .flex_col()
        .when(!first, |heading| {
            heading.child(div().my(px(GAP)).h(px(1.0)).bg(neutral.border))
        })
        .child(
            div()
                .px(px(ACROSS))
                .pt(px(GAP))
                .pb(px(GAP))
                .type_(Type::Meta)
                .line_height(px(fonts::ui(16.0)))
                .text_color(neutral.foreground_muted)
                .child(label.into()),
        )
}

/// The line shown when nothing answers the search.
#[must_use]
pub fn command_empty(text: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .flex()
        .justify_center()
        .px(px(ACROSS))
        .py(px(spacing::S6))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.into())
}
