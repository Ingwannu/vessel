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

use std::ops::Range;

pub mod actions;

/// Optional markdown interaction policy; the old entry point retains plain copy.
#[derive(Debug, Default)]
pub struct Options<'a> {
    /// Existing find state and its version key.
    pub marks: Option<&'a FindMarks<'a>>,
    /// Link and image ownership supplied by the host.
    pub actions: Option<actions::Actions>,
    /// Selection formatting independent of link handling.
    pub copy: actions::CopyStyle,
}

use gpui::{
    AnyElement, App, AppContext as _, ElementId, Entity, Hsla, IntoElement as _, Overflow,
    ParentElement as _, Pixels, Refineable as _, SharedString, StyleRefinement, Styled as _,
    WhiteSpace, Window, div, px, relative,
};
use gpui_base::text::{RangeHighlight, TextView, TextViewState, TextViewStyle};

use crate::copy::{CopyLook, copy_button};
use crate::fonts::Type;
use crate::theme::{Scheme, Tokens};
use crate::widgets::TypeExt as _;

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

/// Find's colours (GOAT's `::highlight(goat-thread-find)` and
/// `goat-thread-find-active`, `index.css`, their oklch in sRGB): every
/// match washed yellow, the current one amber. Not neutral colours, so
/// not the kit's neutral steps.
#[derive(Debug, Clone, Copy)]
pub struct FindPalette {
    /// Behind every match: oklch(0.915 0.11 85) light, oklch(0.55 0.1
    /// 85) dark.
    pub found: Hsla,
    /// Behind the current one: oklch(0.774 0.152 74.3).
    pub current: Hsla,
    /// The current one's ink, where the text's colour can be set (a plain
    /// text; a reply's renderer paints backgrounds only): oklch(0.216
    /// 0.028 77.4).
    pub current_ink: Hsla,
}

impl FindPalette {
    /// The palette for `tokens`' scheme.
    #[must_use]
    pub fn of(tokens: &Tokens) -> Self {
        let found = match tokens.scheme {
            Scheme::Light => gpui::rgb(0x00ff_de8b),
            Scheme::Dark => gpui::rgb(0x008c_6c1f),
        };
        Self {
            found: found.into(),
            current: gpui::rgb(0x00ee_a52b).into(),
            current_ink: gpui::rgb(0x0021_180a).into(),
        }
    }
}

/// What find marks in a reply.
pub struct FindMarks<'a> {
    /// Where the reply's list scrolls the current match into view (the
    /// renderer cannot scroll a list it does not own).
    pub list: &'a crate::VirtualList,
    /// What to keep clear at the list's top when scrolling there (the
    /// find bar floats over it).
    pub clear_top: Pixels,
    /// Changes whenever the query or the current match does: the marks
    /// are worked out again only then (and when the text changes).
    pub key: u64,
    /// Where the query occurs in a text, as byte ranges of it (the view's
    /// `timeline::spans_of`), run on the text as the reply is drawn.
    pub spans: &'a dyn Fn(&str) -> Vec<Range<usize>>,
    /// Which of them is the current match: marked apart and scrolled to.
    pub current: Option<usize>,
}

impl std::fmt::Debug for FindMarks<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FindMarks")
            .field("key", &self.key)
            .field("current", &self.current)
            .finish_non_exhaustive()
    }
}

/// A reply's renderer state, kept across frames by the reply's id.
struct Kept {
    state: Entity<TextViewState>,
    /// The version of the text it holds.
    version: u64,
    /// The marks' key it shows; `None`: none shown.
    marked: Option<u64>,
}

/// A reply's Markdown, its code blocks drawn as GOAT draws them, find's
/// matches marked when `marks` says. `id` keys the reply's renderer
/// state and must stay the same for the same reply across frames;
/// `version` changes whenever `text` does (an entry's revision).
pub fn chat_markdown(
    id: impl Into<ElementId>,
    text: &str,
    version: u64,
    marks: Option<&FindMarks<'_>>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    chat_markdown_with(
        id,
        text,
        version,
        Options {
            marks,
            ..Options::default()
        },
        tokens,
        window,
        cx,
    )
}

/// The same retained renderer with host-owned links/images and code-aware copy.
/// Find, source versioning, styling and code controls use the existing path.
pub fn chat_markdown_with(
    id: impl Into<ElementId>,
    text: &str,
    version: u64,
    options: Options<'_>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let marks = options.marks;
    let look = Look::of(tokens);
    let palette = FindPalette::of(tokens);
    let kept = window.use_keyed_state(id.into(), cx, |_, cx| Kept {
        state: cx.new(|cx| TextViewState::markdown(text, cx)),
        version,
        marked: None,
    });
    let state = kept.update(cx, |kept, cx| {
        if kept.version != version {
            kept.version = version;
            kept.state.update(cx, |state, cx| state.set_text(text, cx));
            kept.marked = None;
        }
        let state = kept.state.clone();
        match marks {
            Some(marks) if kept.marked != Some(marks.key) => {
                kept.marked = state.update(cx, |state, cx| mark(state, marks, palette, cx));
            }
            None if kept.marked.is_some() => {
                state.update(cx, TextViewState::clear_range_highlights);
                kept.marked = None;
            }
            Some(_) | None => {}
        }
        state
    });
    let list = marks.map(|marks| (marks.list.clone(), marks.clear_top));
    let view = TextView::new(&state)
        .on_reveal(move |bounds, _window, _cx| {
            if let Some((list, clear_top)) = &list {
                list.reveal_bounds(bounds, *clear_top);
            }
        })
        .style(presentation(look, cx))
        .code_block_actions(move |code, window, cx| {
            corner(code.code().as_ref(), code.lang(), look, window, cx)
        });
    actions::finish(view, state, options.actions, options.copy)
}

/// Component installs its theme on the same Base renderer. Refine that installed
/// style directly so new engine callbacks keep the existing palette and geometry.
fn presentation(look: Look, cx: &App) -> TextViewStyle {
    let defaults = gpui_base::TextViewDefaults::global(cx);
    let themed = defaults
        .style()
        .cloned()
        .unwrap_or_else(|| TextViewStyle::from_theme(&gpui_base::Theme::global(cx)));
    let refine = |mut base: StyleRefinement, overlay: StyleRefinement| {
        base.refine(&overlay);
        base
    };
    let code = refine(themed.code_block().clone(), block(look));
    let table = refine(themed.table().clone(), table());
    let head = refine(themed.table_head().clone(), table_head(look));
    let cell = refine(themed.table_cell().clone(), table_cell());
    themed
        .with_code_block(code)
        .with_table(table)
        .with_table_head(head)
        .with_table_cell(cell)
}

/// Mark `marks` in the reply `state` holds and scroll to the current
/// one; the key marked, or `None` while the text is not laid out yet (it
/// is tried again next frame).
pub(crate) fn mark(
    state: &mut TextViewState,
    marks: &FindMarks<'_>,
    palette: FindPalette,
    cx: &mut gpui::Context<'_, TextViewState>,
) -> Option<u64> {
    let rendered = state.rendered_text();
    if rendered.is_empty() {
        return None;
    }
    let spans = (marks.spans)(rendered.as_str());
    let current = marks.current.and_then(|nth| spans.get(nth).cloned());
    let highlights = spans.iter().enumerate().map(|(nth, span)| {
        let color = if Some(nth) == marks.current {
            palette.current
        } else {
            palette.found
        };
        RangeHighlight::new(span.clone(), color)
    });
    // A range the renderer refuses (text outside every block) leaves the
    // reply unmarked rather than failing the frame.
    let _refused = state.set_range_highlights(highlights, cx);
    if let Some(current) = current {
        let _not_taken = state.reveal_range(current, cx);
    }
    Some(marks.key)
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
        .text_size(px(crate::fonts::ui(TABLE_SIZE)));
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
    text: &str,
    lang: Option<SharedString>,
    look: Look,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let text: SharedString = gpui_base::text::trim_code_line_breaks(text)
        .to_owned()
        .into();
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .bg(look.fill)
        .children(lang.filter(|lang| !lang.is_empty()).map(|lang| {
            div()
                .face(Type::Mono)
                .text_size(px(crate::fonts::code(LANGUAGE_SIZE)))
                .text_color(look.quiet.opacity(0.72))
                .child(lang)
        }))
        .child(copy_button("copy", text, look.copy(), window, cx))
        .into_any_element()
}
