//! An agent's reply as GOAT's chat draws it: Markdown whose fenced code
//! sits in a block of its own that scrolls sideways instead of wrapping,
//! with the language and a copy button at its top
//! (`apps/web/src/components/chat/MarkdownCodeBlock.tsx`, `index.css`
//! `.chat-markdown pre` and `.chat-markdown-codeblock`).
//!
//! [Decision Log]
//! - Purpose: a reply's code read as code. The component layer's
//!   default wraps a 240-character line into four ragged rows, puts the
//!   language alone in a corner and offers no copy: a library's default
//!   look, which the owner ruled out, and a broken shape on real replies
//!   (the fleet's `shapes` session, `scripts/fleet.sh`).
//! - Constraint: the renderer (gpui-component over gpui-base 0.8.1) owns
//!   the block's element. A caller can refine its style and put one
//!   element in its top-right corner (`code_block_actions`); it cannot
//!   lay out a header row of its own.
//! - Alternatives: draw code blocks ourselves by splitting a reply into
//!   Markdown and code parts (a selection would then stop at every
//!   block, and streaming would re-split the reply each chunk); keep the
//!   kit's plain [`crate::markdown`] (the default look).
//! - Choice: refine the block to GOAT's measures (radius 8, padding
//!   0.9rem by 0.8rem under a header's height, `leading-snug`) and keep
//!   its lines whole (`white-space: nowrap`) in a row as wide as the
//!   longest line. The block scrolls on both axes: that takes the
//!   renderer's scrolled branch, which keeps the corner outside what
//!   scrolls; with no height cap nothing scrolls vertically, and the
//!   branch's mask hands the wheel's vertical turn to the timeline. The
//!   corner carries the language (mono, 11 px, GOAT's `text-2xs`, at 72%
//!   of the text) and the copy button ([`crate::copy`]). Tables lose
//!   their frame, fill and column rules for rules under the rows and a
//!   semibold header at 14 px (GOAT's `.chat-markdown table`); links take
//!   the theme's link colour, set from the tokens (`field::apply_tokens`).
//! - Constraint kept when changing this: a style passed here is folded
//!   onto one the renderer derives from the component layer's theme on
//!   every layout, so every colour a reply shows must be either refined
//!   here or set on that theme from the tokens; a colour left to the
//!   component layer's default shows its light theme in the dark one.
//! - Differences from GOAT, kept: the language sits beside the copy button
//!   at the right, not alone at the left (the renderer's one corner); the
//!   button is always shown, not revealed on hover (the block's element is
//!   not ours to mark as a hover group); a table's outer cells keep their
//!   inset (GOAT's sit flush with the prose); no wrap toggle and no "run
//!   in terminal" yet (§7, the terminal, comes later); code is not
//!   highlighted (the renderer's highlighter needs a grammar feature the
//!   kit does not build).

use gpui::{
    AnyElement, App, ElementId, Hsla, IntoElement, Overflow, ParentElement as _, SharedString,
    StyleRefinement, Styled as _, WhiteSpace, Window, div, px, relative,
};
use gpui_component::text::{TextView, TextViewStyle};

use crate::copy::{CopyLook, copy_button};
use crate::fonts::MONO;
use crate::theme::{Scheme, Tokens};

/// The block's corner radius (GOAT's `rounded-lg`).
const RADIUS: f32 = 8.0;

/// The code's inset (GOAT's `pre` padding, 0.9rem by 0.8rem).
const PAD_X: f32 = 14.0;
const PAD_Y: f32 = 13.0;

/// The code's top inset: a header's height (GOAT's `pt-1.5` and an
/// `icon-xs` button, 26), where the corner element sits, over the code's
/// own inset (13).
const PAD_TOP: f32 = 39.0;

/// The language's size (GOAT's `text-2xs`, 0.6875rem).
const LANGUAGE_SIZE: f32 = 11.0;

/// A table's text size (GOAT's 0.875rem in its new design).
const TABLE_SIZE: f32 = 14.0;

/// The colours code is drawn in, from the kit's neutral steps: a reply's
/// code blocks and a tool's text alike (GOAT draws both on its code
/// tokens).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Look {
    /// The block: GOAT's `--secondary` (#f9f9f9) in light, its
    /// `--code-background` (darker than the page) in dark.
    pub(crate) fill: Hsla,
    /// Its edge: `border` in light, none in dark (GOAT's
    /// `dark:border-transparent`).
    pub(crate) edge: Hsla,
    /// The language and the button's glyph.
    pub(crate) quiet: Hsla,
    /// The button under the pointer.
    pub(crate) hover: Hsla,
    /// The text.
    pub(crate) text: Hsla,
}

impl Look {
    pub(crate) fn of(tokens: &Tokens) -> Self {
        let neutral = &tokens.neutral;
        let (fill, edge) = match tokens.scheme {
            Scheme::Light => (neutral.surface1, neutral.border),
            Scheme::Dark => (neutral.surface_sidebar, gpui::transparent_black()),
        };
        Self {
            fill,
            edge,
            quiet: neutral.foreground_muted,
            hover: neutral.surface2,
            text: neutral.foreground,
        }
    }

    /// The copy button's colours.
    pub(crate) const fn copy(self) -> CopyLook {
        CopyLook {
            glyph: self.quiet,
            hover: self.hover,
        }
    }
}

/// A reply's Markdown, its code blocks drawn as GOAT draws them. `id`
/// keys the renderer's cache and must stay the same for the same text
/// across frames.
#[must_use]
pub fn chat_markdown(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
    tokens: &Tokens,
) -> impl IntoElement {
    let look = Look::of(tokens);
    TextView::markdown(id, text)
        .style(
            TextViewStyle::default()
                .code_block(block(look))
                .table(table())
                .table_head(table_head(look))
                .table_cell(table_cell()),
        )
        .code_block_actions(move |code, window, cx| {
            corner(code.code(), code.lang(), look, window, cx)
        })
}

/// The block's style over the renderer's.
fn block(look: Look) -> StyleRefinement {
    let mut style = StyleRefinement::default()
        .bg(look.fill)
        .border_color(look.edge)
        .rounded(px(RADIUS))
        .px(px(PAD_X))
        .pt(px(PAD_TOP))
        .pb(px(PAD_Y))
        .line_height(relative(1.375));
    // Scrolling on both axes takes the renderer's scrolled branch, which
    // keeps the corner outside what scrolls; with no height cap nothing
    // scrolls vertically, and its mask passes the wheel to the timeline.
    // A row lets the unwrapped code be as wide as its longest line, so
    // the block has something to scroll.
    style.overflow.x = Some(Overflow::Scroll);
    style.overflow.y = Some(Overflow::Scroll);
    style.restrict_scroll_to_axis = Some(true);
    style.display = Some(gpui::Display::Flex);
    style.flex_direction = Some(gpui::FlexDirection::Row);
    style.text.white_space = Some(WhiteSpace::Nowrap);
    style
}

/// A table: no frame and no fill, rules under the rows only, at 14
/// pixels (GOAT's `.chat-markdown table`, one step under the prose).
///
/// Narrower than its columns' floors (a phone), it scrolls sideways
/// instead of breaking words letter by letter: the renderer's adaptive
/// layout, GOAT's "the min column width is the longest word, and only
/// wider content scrolls".
fn table() -> StyleRefinement {
    let mut style = StyleRefinement::default()
        .bg(gpui::transparent_black())
        .border_color(gpui::transparent_black())
        .rounded(px(0.0))
        .text_size(px(TABLE_SIZE));
    style.overflow.x = Some(Overflow::Scroll);
    style
}

/// The header row: no fill, semibold (GOAT's `thead th`).
fn table_head(look: Look) -> StyleRefinement {
    StyleRefinement::default()
        .bg(gpui::transparent_black())
        .text_color(look.text)
        .font_weight(gpui::FontWeight::SEMIBOLD)
}

/// A cell: no rule between columns (the renderer draws one in the
/// cell's border colour).
fn table_cell() -> StyleRefinement {
    StyleRefinement::default().border_color(gpui::transparent_black())
}

/// The block's corner: its language and the copy button.
fn corner(
    text: SharedString,
    lang: Option<SharedString>,
    look: Look,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .bg(look.fill)
        .children(lang.filter(|lang| !lang.is_empty()).map(|lang| {
            div()
                .font_family(MONO)
                .text_size(px(LANGUAGE_SIZE))
                .text_color(look.quiet.opacity(0.72))
                .child(lang)
        }))
        .child(copy_button("copy", text, look.copy(), window, cx))
        .into_any_element()
}
