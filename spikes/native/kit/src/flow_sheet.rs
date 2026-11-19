//! A flow sheet: Paseo's Add project panel (`add-project-flow.tsx` styles,
//! pinned 0.15.4; the page's `ui/kit/flow-sheet.tsx`). A panel at the top
//! of the window over black at 50%: a header with the page's title row (a
//! back arrow on later pages, the title, a muted context) and its field,
//! then the page's rows, then the key hints.
//!
//! [Decision Log]
//! - Purpose: Add project's pages (TRACKER 234) drawn as Paseo's.
//! - Constraint: the command centre's frame (`command`) is Paseo's
//!   command centre: a fixed 560 height, no title row, no footer, and a
//!   bottom sheet on a phone; Paseo's Add project is its own modal with
//!   none of those.
//! - Alternatives: options on the command frame (two looks behind one
//!   function, each caller passing what the other ignores).
//! - Choice: a frame of its own with Paseo's values: the panel 640 wide,
//!   at most 92% by 80% of the window, 48 under its top, a hairline of
//!   `border`, 8 round, `surface0`, the large shadow; the header 16
//!   across, 8 above, 12 below, 8 between its rows, over a rule; the title
//!   row at least 20 high, its back mark in an 18 by 20 slot, the title and
//!   the context 12 on one baseline, 8 apart; the rows' area 8 above and
//!   below; a row 16 across, 8 down, its mark in an 18 slot 12 from its
//!   words, the title 14 in the foreground, the second line 12 muted 2
//!   under it, `surface1` when active or hovered, half opacity when it
//!   cannot be chosen; the footer 16 across, 8 down, over a rule, its
//!   hints 16 apart, a hint's keys in one badge (Paseo's `Shortcut`: 4 by
//!   2 inside, 6 round, `surface3` at half) 12 in the foreground, 6 from
//!   its action 12 muted. Up, Down, Enter and Escape reach the owner before the field
//!   takes them, as in the command centre.
//! - Must stay true: a press on the panel never reaches the scrim; the
//!   owner keeps the page, the active row and focus.

use gpui::prelude::*;
use gpui::{
    AnyElement, Context, Div, ElementId, MouseButton, ScrollHandle, SharedString, Stateful,
    Window, anchored, deferred, div, point, px, relative, rgba,
};
use gpui_base::input::{Enter, Escape, MoveDown, MoveUp};

use crate::assets::Glyph;
use crate::command::{CommandKey, WIDTH, shadow_lg};
use crate::fonts::Type;
use crate::picker::glyph;
use crate::popup::MENU_RADIUS;
use crate::theme::{COMMAND_SCRIM, Tokens, spacing};
use crate::widgets::TypeExt as _;

/// Paseo's back-button slot width and a mark's slot width.
const SLOT: f32 = 18.0;
/// Paseo's `iconSize.lg`: the title row's least height.
const TITLE_ROW: f32 = 20.0;
/// Paseo's `borderRadius.md`: a key badge's corners.
const BADGE_RADIUS: f32 = 6.0;

/// One key hint in the footer: its keys, as Paseo's `formatShortcut`
/// writes them (`↑↓` on a Mac, `↑+↓` elsewhere, `⏎`, `Esc`), and what they
/// do.
#[derive(Debug, Clone)]
pub struct FlowHint {
    /// The keys, in one badge.
    pub keys: SharedString,
    /// What they do.
    pub action: SharedString,
}

/// What the sheet holds.
pub struct FlowSheetFrame {
    /// The page's title.
    pub title: SharedString,
    /// The machine or other context beside it, muted.
    pub context: Option<SharedString>,
    /// Whether the back arrow shows (a later page).
    pub back: bool,
    /// The page's field; on a page without one it stays, hidden, to keep
    /// the keys (Paseo's `keyboardCapture`, 1 by 1 at no opacity).
    pub field: AnyElement,
    /// Whether the page shows its field.
    pub field_shown: bool,
    /// The state lines and rows, in order; each row a child, so the
    /// owner can scroll the active one into view.
    pub results: Vec<AnyElement>,
    /// The results' scroll position.
    pub scroll: ScrollHandle,
    /// The footer's hints.
    pub hints: Vec<FlowHint>,
}

impl std::fmt::Debug for FlowSheetFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlowSheetFrame")
            .field("title", &self.title)
            .field("back", &self.back)
            .finish_non_exhaustive()
    }
}

/// The owner's keys, its back arrow, and a press outside.
pub struct FlowHandlers<V: 'static> {
    /// Up, Down, Enter, Escape, before the field takes them.
    pub on_key: fn(&mut V, CommandKey, &mut Window, &mut Context<'_, V>),
    /// The back arrow.
    pub back: fn(&mut V, &mut Window, &mut Context<'_, V>),
    /// A press on the scrim.
    pub close: fn(&mut V, &mut Window, &mut Context<'_, V>),
}
impl<V: 'static> Copy for FlowHandlers<V> {}
impl<V: 'static> Clone for FlowHandlers<V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<V: 'static> std::fmt::Debug for FlowHandlers<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlowHandlers").finish_non_exhaustive()
    }
}

fn keys<V: 'static>(panel: Stateful<Div>, handlers: FlowHandlers<V>, cx: &mut Context<'_, V>) -> Stateful<Div> {
    panel
        .capture_action(cx.listener(move |view: &mut V, _: &MoveUp, window, cx| {
            cx.stop_propagation();
            (handlers.on_key)(view, CommandKey::Up, window, cx);
        }))
        .capture_action(cx.listener(move |view: &mut V, _: &MoveDown, window, cx| {
            cx.stop_propagation();
            (handlers.on_key)(view, CommandKey::Down, window, cx);
        }))
        .capture_action(cx.listener(move |view: &mut V, _: &Enter, window, cx| {
            cx.stop_propagation();
            (handlers.on_key)(view, CommandKey::Enter, window, cx);
        }))
        .capture_action(cx.listener(move |view: &mut V, _: &Escape, window, cx| {
            cx.stop_propagation();
            (handlers.on_key)(view, CommandKey::Escape, window, cx);
        }))
}

fn title_row<V: 'static>(
    frame: &FlowSheetFrame,
    handlers: FlowHandlers<V>,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> Div {
    let neutral = tokens.neutral;
    let back = frame.back.then(|| {
        div()
            .id("flow-sheet-back")
            .w(px(SLOT))
            .h(px(TITLE_ROW))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(glyph(Glyph::ArrowLeft, spacing::S4, neutral.foreground_muted))
            .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
                (handlers.back)(view, window, cx);
            }))
    });
    let titles = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_row()
        .items_baseline()
        .gap(px(spacing::S2))
        .type_(Type::Meta)
        .child(
            div()
                .min_w_0()
                .flex_shrink()
                .truncate()
                .text_color(neutral.foreground)
                .child(frame.title.clone()),
        )
        .children(frame.context.clone().map(|context| {
            div()
                .min_w_0()
                .flex_shrink()
                .truncate()
                .text_color(neutral.foreground_muted)
                .child(context)
        }));
    div()
        .min_h(px(TITLE_ROW))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .children(back)
        .child(titles)
}

fn footer(hints: &[FlowHint], tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex_none()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S4))
        .px(px(spacing::S4))
        .py(px(spacing::S2))
        .border_t_1()
        .border_color(neutral.border)
        .type_(Type::Meta)
        .children(hints.iter().map(|hint| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S1_5))
                .child(
                    div()
                        .px(px(spacing::S1))
                        .py(px(spacing::S0_5))
                        .rounded(px(BADGE_RADIUS))
                        .bg(neutral.surface3.opacity(0.5))
                        .text_color(neutral.foreground)
                        .child(hint.keys.clone()),
                )
                .child(
                    div()
                        .text_color(neutral.foreground_muted)
                        .child(hint.action.clone()),
                )
        }))
}

/// The sheet over the window.
pub fn flow_sheet<V: 'static>(
    frame: FlowSheetFrame,
    handlers: FlowHandlers<V>,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let header = div()
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .px(px(spacing::S4))
        .pt(px(spacing::S2))
        .pb(px(spacing::S3))
        .border_b_1()
        .border_color(neutral.border)
        .relative()
        .child(title_row(&frame, handlers, tokens, cx))
        .child(if frame.field_shown {
            div().py(px(spacing::S1)).type_(Type::Body).child(frame.field)
        } else {
            div()
                .absolute()
                .w(px(1.0))
                .h(px(1.0))
                .overflow_hidden()
                .opacity(0.0)
                .child(frame.field)
        });
    let results = div()
        .id("flow-sheet-results")
        .flex_shrink()
        .min_h(px(0.0))
        .overflow_y_scroll()
        .track_scroll(&frame.scroll)
        .py(px(spacing::S2))
        .children(frame.results);
    let panel = div()
        .id("flow-sheet")
        .w(px(WIDTH))
        .max_w(relative(0.92))
        .max_h(relative(0.8))
        .flex()
        .flex_col()
        .overflow_hidden()
        .border_1()
        .border_color(neutral.border)
        .rounded(px(MENU_RADIUS))
        .shadow(shadow_lg(tokens))
        .bg(neutral.surface0)
        .type_(Type::Body)
        .text_color(neutral.foreground)
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .child(header)
        .child(results)
        .child(footer(&frame.hints, tokens));
    let panel = keys(panel, handlers, cx);
    let size = window.viewport_size();
    let scrim = div()
        .id("flow-sheet-scrim")
        .w(size.width)
        .h(size.height)
        .flex()
        .flex_col()
        .items_center()
        .justify_start()
        .pt(px(spacing::S12))
        .bg(rgba(COMMAND_SCRIM))
        .occlude()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view: &mut V, _event, window, cx| {
                (handlers.close)(view, window, cx);
                cx.notify();
            }),
        )
        .child(panel);
    // Over a dialog (at 2), under a tooltip (at 4), as the command centre.
    deferred(anchored().position(point(px(0.0), px(0.0))).child(scrim))
        .with_priority(3)
        .into_any_element()
}

/// A row's parts.
pub struct FlowRow {
    /// Its mark, drawn in the 18 slot.
    pub mark: AnyElement,
    /// Its first line.
    pub title: SharedString,
    /// Its second line.
    pub subtitle: Option<SharedString>,
    /// The keyboard's row.
    pub active: bool,
    /// It cannot be chosen.
    pub disabled: bool,
}

impl std::fmt::Debug for FlowRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlowRow")
            .field("title", &self.title)
            .field("active", &self.active)
            .field("disabled", &self.disabled)
            .finish_non_exhaustive()
    }
}

/// One row; the owner adds its click.
pub fn flow_row(id: impl Into<ElementId>, row: FlowRow, tokens: &Tokens) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let wash = neutral.surface1;
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S3))
        .px(px(spacing::S4))
        .py(px(spacing::S2))
        .when(row.active, |drawn| drawn.bg(wash))
        .when(row.disabled, |drawn| drawn.opacity(0.5))
        .when(!row.disabled, |drawn| {
            drawn.cursor_pointer().hover(move |style| style.bg(wash))
        })
        .child(
            div()
                .w(px(SLOT))
                .flex_none()
                .flex()
                .justify_center()
                .text_color(neutral.foreground_muted)
                .child(row.mark),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .type_(Type::Body)
                        .text_color(neutral.foreground)
                        .child(row.title),
                )
                .children(row.subtitle.map(|subtitle| {
                    div()
                        .mt(px(spacing::S0_5))
                        .truncate()
                        .type_(Type::Meta)
                        .text_color(neutral.foreground_muted)
                        .child(subtitle)
                })),
        )
}

/// A state line ("Loading...", the empty words, the progress): 14 muted,
/// 16 all round.
#[must_use]
pub fn flow_state(text: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .p(px(spacing::S4))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.into())
}

/// The new folder's path under the name field: 14 muted, 16 by 8.
#[must_use]
pub fn flow_preview(text: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .px(px(spacing::S4))
        .py(px(spacing::S2))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.into())
}

/// A failure: 12 in the destructive colour, 16 by 12.
#[must_use]
pub fn flow_error(text: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .px(px(spacing::S4))
        .py(px(spacing::S3))
        .type_(Type::Meta)
        .text_color(tokens.neutral.destructive)
        .child(text.into())
}
