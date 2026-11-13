//! Pickers, as Paseo's composer draws them: the pill a picker is opened
//! from, the list it opens (a combobox: a search row, rows with a
//! description, a checked row, rows that open a list of their own), and
//! the round send button beside them. Every picker in the app is drawn
//! with these (the owner on 0.0.0.0.3: "앞으로 드롭다운에도 그러지말고").
//!
//! [Decision Log]
//! - Purpose: the tray's pickers were words with a "▾" (a character in
//!   neither of the app's faces), not controls; the owner named Paseo's.
//! - Reference: Paseo's `composer/agent-controls/control.tsx` (the pill),
//!   `components/ui/combobox.tsx` (the list), `composer/input/input.tsx`
//!   (the send button); values in `refs/notes/ui-paseo-goat-spec.md` B1,
//!   A11 and section 2.1, each checked against the source it cites.
//! - The pill: 28 tall (44 on touch), 8 across, glyph and words 4 apart,
//!   16 round, clear at rest, `surface2` hovered, `surface0` while open;
//!   the value in 14 regular `foregroundMuted`; glyph 14 (20 on touch);
//!   a 14 caret.
//! - The list: on the floating surface track C's `popup.rs` shares
//!   (`Surfaces`, `shadow_md`, `MENU_RADIUS`), `surface0` with a hairline of
//!   `border`, at most 400 tall, opening 5 above its trigger, its left edge
//!   on the trigger's; a search row on `surface1` with a hairline under it;
//!   rows at least 36 (44 on touch), 12 by 8, label 14, description 12
//!   muted, the chosen row's muted check at the right; a hovered or
//!   keyboard-highlighted row on `surface1`; a disabled row at 55%.
//!   Arrows (wrapping), Enter, Escape and a click outside. Below Paseo's
//!   compact width (600) it is a sheet from the bottom (`surface0`, 16
//!   round on top, the page behind dimmed to 45%), as Paseo's.
//! - Choice: like track C's menu, the state is a value the screen keeps
//!   and the list is drawn from it each frame; the rows are the screen's,
//!   rebuilt from the search's text, and a chosen row comes back by index.
//! - Cost: no type-ahead without the search row (Paseo has none either),
//!   no favourites (GOAT's), no animation.

use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    Anchor, AnyElement, App, Bounds, Context, Div, ElementId, FocusHandle, Hsla, KeyDownEvent,
    MouseButton, Pixels, SharedString, Stateful, Window, anchored, canvas, deferred, div, point,
    px, rgb, rgba, svg,
};

use crate::assets::Glyph;
use crate::field::TextField;
use crate::fonts::Type;
use crate::popup::{self, Surfaces};
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// Paseo's compact width: below it pickers are sheets and controls are
/// touch-sized (`constants/layout.ts`).
pub const COMPACT: f32 = 600.0;

/// How far above its trigger a list opens (up, so negative).
const LIFT: f32 = -5.0;

/// The tallest a list is.
const MAX_HEIGHT: f32 = 400.0;

/// A glyph, `size` square, in `color`.
#[must_use]
pub fn glyph(which: Glyph, size: f32, color: Hsla) -> gpui::Svg {
    svg()
        .path(which.path())
        .size(px(size))
        .flex_shrink_0()
        .text_color(color)
}

/// A picker's pill: `glyph`, the value, a quieter second value (the
/// effort beside the model), a caret. `open`: its list is showing.
#[must_use]
pub fn pill(
    id: impl Into<ElementId>,
    leading: Option<Glyph>,
    label: impl Into<SharedString>,
    quiet: Option<SharedString>,
    open: bool,
    touch: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let glyph_size = if touch { 20.0 } else { 14.0 };
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .flex_shrink_0()
        .gap(px(4.0))
        .h(px(if touch { 44.0 } else { 28.0 }))
        .px(px(8.0))
        .rounded(px(16.0))
        .when(open, |pill| pill.bg(neutral.surface0))
        .when(!open, |pill| {
            pill.hover(move |style| style.bg(neutral.surface2))
        })
        .cursor_pointer()
        .type_(Type::Body)
        .text_color(neutral.foreground_muted)
        .children(leading.map(|leading| glyph(leading, glyph_size, neutral.foreground_muted)))
        .child(label.into())
        .children(quiet.map(|quiet| {
            div()
                .text_color(neutral.foreground_extra_muted)
                .child(quiet)
        }))
        .child(glyph(Glyph::ChevronDown, 14.0, neutral.foreground_muted))
}

/// What the round button beside the composer does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendKind {
    /// Send the text (an arrow up, on the accent).
    Send,
    /// Stop the running turn (a square, on red).
    Stop,
}

/// Paseo's send button: a circle 28 across (44 on touch), the accent with
/// a white arrow, half-faded while there is nothing to send; while a turn
/// runs and nothing is typed, a red circle with a white square.
#[must_use]
pub fn send_button(
    id: impl Into<ElementId>,
    kind: SendKind,
    enabled: bool,
    touch: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    // Paseo's stop is `palette.red[600]`, the same in both themes.
    let (fill, mark) = match kind {
        SendKind::Send => (neutral.accent, Glyph::ArrowUp),
        SendKind::Stop => (rgb(0x00dc_2626).into(), Glyph::Stop),
    };
    let size = if touch { 44.0 } else { 28.0 };
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .size(px(size))
        .rounded_full()
        .bg(fill)
        .when(!enabled, |button| button.opacity(0.5))
        .when(enabled, Styled::cursor_pointer)
        .child(glyph(
            mark,
            if touch { 20.0 } else { 16.0 },
            neutral.accent_foreground,
        ))
}

/// What leads a row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Leading {
    /// A glyph, muted.
    Glyph(Glyph),
    /// A state's dot (a machine).
    Dot(Hsla),
}

/// One row of a list.
#[derive(Debug, Clone, PartialEq)]
pub struct ComboRow {
    /// Its words.
    pub label: SharedString,
    /// A line under them, muted (a mode's description, why an agent
    /// cannot start).
    pub detail: Option<SharedString>,
    /// What leads it.
    pub leading: Option<Leading>,
    /// Words at its right, muted ("3 models", "Loading").
    pub trailing: Option<SharedString>,
    /// It opens a list of its own (a chevron at its right).
    pub opens: bool,
    /// It is the one in effect (a check at its right).
    pub checked: bool,
    /// It cannot be chosen now.
    pub disabled: bool,
    /// A heading drawn above it (a group's name).
    pub heading: Option<SharedString>,
}

impl ComboRow {
    /// A plain row.
    #[must_use]
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            detail: None,
            leading: None,
            trailing: None,
            opens: false,
            checked: false,
            disabled: false,
            heading: None,
        }
    }
}

/// What sits above the rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComboHeader {
    /// A title row, with a way back when `back` (a provider's models);
    /// `None` for none.
    pub title: Option<SharedString>,
    /// The title row leads back.
    pub back: bool,
    /// A search row (its placeholder is given when the list opens).
    pub search: bool,
    /// Shown when no row is left (the search matched nothing).
    pub empty: SharedString,
}

/// A list's state, kept by the screen that shows it.
#[derive(Debug)]
pub struct ComboboxState {
    open: bool,
    highlighted: Option<usize>,
    focus: FocusHandle,
    search: TextField,
    /// Where its trigger was last drawn, in the window.
    anchor: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl ComboboxState {
    /// A closed list; its search row redraws its owner as it is typed in.
    pub fn new<V: 'static>(window: &mut Window, cx: &mut Context<'_, V>) -> Self {
        let search = TextField::with_change(
            "",
            |_owner: &mut V, _text, _window, _cx| {},
            |_owner: &mut V, _text, cx| cx.notify(),
            window,
            cx,
        );
        Self {
            open: false,
            highlighted: None,
            focus: cx.focus_handle(),
            search,
            anchor: Rc::new(Cell::new(None)),
        }
    }

    /// Open it, its search emptied and holding the keys, showing `search`
    /// while empty (or the list holding the keys, with no search row).
    pub fn open(&mut self, search: Option<&str>, window: &mut Window, cx: &mut App) {
        self.open = true;
        self.highlighted = None;
        self.search.clear(window, cx);
        if let Some(placeholder) = search {
            self.search.set_placeholder(placeholder, window, cx);
            self.search.focus(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
    }

    /// Close it.
    pub const fn close(&mut self) {
        self.open = false;
        self.highlighted = None;
    }

    /// Whether it is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.open
    }

    /// What is typed in its search row.
    #[must_use]
    pub fn query(&self, cx: &App) -> String {
        self.search.text(cx)
    }

    /// Start its search over, showing `placeholder` while empty (a way
    /// back, a row's own list).
    pub fn reset_query(&mut self, placeholder: &str, window: &mut Window, cx: &mut App) {
        self.highlighted = None;
        self.search.clear(window, cx);
        self.search.set_placeholder(placeholder, window, cx);
        self.search.focus(window, cx);
    }

    /// Put this in the trigger: it notes where the trigger is drawn, for
    /// the list to open from.
    #[must_use]
    pub fn anchor(&self) -> AnyElement {
        let cell = Rc::clone(&self.anchor);
        canvas(
            move |bounds, _window, _cx| cell.set(Some(bounds)),
            |_, (), _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .into_any_element()
    }

    fn step(&mut self, rows: &[ComboRow], forward: bool) {
        let count = rows.len();
        if count == 0 {
            return;
        }
        let mut at = self.highlighted;
        for _ in 0..count {
            let next = match (at, forward) {
                (None, true) => 0,
                (None | Some(0), false) => count.saturating_sub(1),
                (Some(index), true) => index.saturating_add(1).checked_rem(count).unwrap_or(0),
                (Some(index), false) => index.saturating_sub(1),
            };
            at = Some(next);
            if rows.get(next).is_some_and(|row| !row.disabled) {
                self.highlighted = at;
                return;
            }
        }
    }
}

/// What a screen does with a chosen row, by index.
pub type Chose<V> = Rc<dyn Fn(&mut V, usize, &mut Window, &mut Context<'_, V>)>;

/// What a screen does on the title row's way back.
pub type Back<V> = Rc<dyn Fn(&mut V, &mut Window, &mut Context<'_, V>)>;

/// What a screen hands the list.
pub struct ComboHandlers<V> {
    /// The screen's list state.
    pub state: fn(&mut V) -> &mut ComboboxState,
    /// A row was chosen (the list stays open; the screen closes it when
    /// the choice is final).
    pub chose: Chose<V>,
    /// The way back from a row's own list.
    pub back: Option<Back<V>>,
}

impl<V> std::fmt::Debug for ComboHandlers<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComboHandlers").finish_non_exhaustive()
    }
}

/// The list, when open: over everything, above its trigger, `width`
/// wide; a sheet from the bottom below the compact width.
#[expect(
    clippy::too_many_arguments,
    reason = "a list is drawn from its state, its content, its handlers and the window's size"
)]
pub fn combobox<V: 'static>(
    id: impl Into<ElementId>,
    state: &ComboboxState,
    header: &ComboHeader,
    rows: &[ComboRow],
    handlers: &ComboHandlers<V>,
    width: f32,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    if !state.open {
        return None;
    }
    let compact = window.viewport_size().width < px(COMPACT);
    let surfaces = Surfaces::of(tokens);
    let neutral = tokens.neutral;
    let get = handlers.state;
    let on_key = key_handler(rows, handlers, cx);
    let on_outside = cx.listener(move |view: &mut V, _event, _window, cx| {
        get(view).close();
        cx.notify();
    });

    let mut panel = div()
        .id(id.into())
        .track_focus(&state.focus)
        .capture_key_down(on_key)
        .occlude()
        .flex()
        .flex_col()
        .bg(surfaces.surface0)
        .border_1()
        .border_color(neutral.border)
        .overflow_hidden()
        .type_(Type::Body)
        .text_color(surfaces.foreground);
    panel = if compact {
        panel
            .w_full()
            .max_h(gpui::relative(0.9))
            .rounded_t(px(popup::SHEET_RADIUS))
            .pb(px(16.0))
    } else {
        panel
            .w(px(width))
            .max_h(px(MAX_HEIGHT))
            .rounded(px(popup::MENU_RADIUS))
            .shadow(popup::shadow_md(tokens))
            .on_mouse_down_out(on_outside)
    };
    if let Some(title) = &header.title {
        panel = panel.child(title_row(title.clone(), header.back, handlers, tokens, cx));
    }
    if header.search {
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(12.0))
                .bg(surfaces.surface1)
                .border_b_1()
                .border_color(neutral.border)
                .child(glyph(Glyph::Search, 14.0, neutral.foreground_muted))
                .child(div().flex_1().child(state.search.bare(tokens, cx))),
        );
    }
    panel = panel.child(rows_list(
        header, rows, state, handlers, compact, tokens, cx,
    ));

    let element = if compact {
        // A sheet: the page dimmed behind, the list from the bottom.
        let size = window.viewport_size();
        let on_scrim = cx.listener(move |view: &mut V, _event, _window, cx| {
            get(view).close();
            cx.notify();
        });
        anchored().position(point(px(0.0), px(0.0))).child(
            div()
                .w(size.width)
                .h(size.height)
                .flex()
                .flex_col()
                .justify_end()
                .bg(rgba(0x0000_0073))
                .on_mouse_down(MouseButton::Left, on_scrim)
                .child(panel),
        )
    } else {
        let at = state.anchor.get().map_or_else(
            // Not drawn yet (the first frame): near the window's middle.
            || point(px(8.0), px(240.0)),
            |bounds| point(bounds.left(), bounds.top()),
        );
        anchored()
            .position(at)
            .offset(point(px(0.0), px(LIFT)))
            .anchor(Anchor::BottomLeft)
            .snap_to_window_with_margin(px(8.0))
            .child(panel)
    };
    Some(deferred(element).with_priority(1).into_any_element())
}

/// Arrows (wrapping), Enter, Escape, and Backspace in an empty search to
/// go back, taken before the search row sees them.
fn key_handler<V: 'static>(
    rows: &[ComboRow],
    handlers: &ComboHandlers<V>,
    cx: &mut Context<'_, V>,
) -> impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static {
    let get = handlers.state;
    let key_rows: Rc<[ComboRow]> = rows.into();
    let chose = Rc::clone(&handlers.chose);
    let back = handlers.back.clone();
    cx.listener(move |view: &mut V, event: &KeyDownEvent, window, cx| {
        let list = get(view);
        match event.keystroke.key.as_str() {
            "escape" => list.close(),
            "down" => list.step(&key_rows, true),
            "up" => list.step(&key_rows, false),
            "enter" => {
                if let Some(index) = list.highlighted {
                    chose(view, index, window, cx);
                }
            }
            "backspace" if back.is_some() && list.query(cx).is_empty() => {
                if let Some(back) = &back {
                    back(view, window, cx);
                }
            }
            _other => return,
        }
        cx.stop_propagation();
        cx.notify();
    })
}

/// The rows, under their headings, or the words for none.
fn rows_list<V: 'static>(
    header: &ComboHeader,
    rows: &[ComboRow],
    state: &ComboboxState,
    handlers: &ComboHandlers<V>,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let mut list = div()
        .id("combobox-rows")
        .flex()
        .flex_col()
        .py(px(4.0))
        .overflow_y_scroll();
    if rows.is_empty() {
        list = list.child(
            div()
                .px(px(12.0))
                .py(px(8.0))
                .text_color(neutral.foreground_muted)
                .child(header.empty.clone()),
        );
    }
    for (index, row) in rows.iter().enumerate() {
        if let Some(heading) = &row.heading {
            list = list.child(
                div()
                    .px(px(12.0))
                    .pt(px(8.0))
                    .pb(px(4.0))
                    .type_(Type::Meta)
                    .text_color(neutral.foreground_muted)
                    .child(heading.clone()),
            );
        }
        list = list.child(combo_row(index, row, state, handlers, compact, tokens, cx));
    }
    list.into_any_element()
}

/// The title row: a way back when it leads back, the title in 14 medium.
fn title_row<V: 'static>(
    title: SharedString,
    back: bool,
    handlers: &ComboHandlers<V>,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let mut row = div()
        .id("combobox-title")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(12.0))
        .py(px(10.0))
        .border_b_1()
        .border_color(neutral.border)
        .type_(Type::Label)
        .text_color(neutral.foreground);
    if back {
        row = row.child(glyph(Glyph::ChevronLeft, 16.0, neutral.foreground_muted));
        if let Some(go_back) = handlers.back.clone() {
            row = row
                .cursor_pointer()
                .hover(move |style| style.bg(neutral.surface1))
                .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
                    go_back(view, window, cx);
                    cx.notify();
                }));
        }
    }
    row.child(title).into_any_element()
}

fn combo_row<V: 'static>(
    index: usize,
    row: &ComboRow,
    state: &ComboboxState,
    handlers: &ComboHandlers<V>,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let highlighted = state.highlighted == Some(index);
    let wash = neutral.surface1;
    let mut words = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .text_color(neutral.foreground)
                .child(row.label.clone()),
        );
    if let Some(detail) = &row.detail {
        words = words.child(
            div()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(detail.clone()),
        );
    }
    let mut element = div()
        .id(ElementId::NamedInteger(
            "combobox-row".into(),
            u64::try_from(index).unwrap_or(u64::MAX),
        ))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .min_h(px(if compact { 44.0 } else { 36.0 }))
        .px(px(12.0))
        .py(px(8.0))
        .when(row.disabled, |element| element.opacity(0.55))
        .when(highlighted && !row.disabled, |element| element.bg(wash))
        .children(row.leading.map(|leading| {
            match leading {
                Leading::Glyph(which) => {
                    glyph(which, 16.0, neutral.foreground_muted).into_any_element()
                }
                Leading::Dot(color) => div()
                    .size(px(14.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().size(px(6.0)).rounded_full().bg(color))
                    .into_any_element(),
            }
        }))
        .child(words)
        .children(row.trailing.clone().map(|trailing| {
            div()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(trailing)
        }))
        .when(row.checked, |element| {
            element.child(glyph(Glyph::Check, 16.0, neutral.foreground_muted))
        })
        .when(row.opens, |element| {
            element.child(glyph(Glyph::ChevronRight, 14.0, neutral.foreground_muted))
        });
    if !row.disabled {
        let chose = Rc::clone(&handlers.chose);
        element = element
            .cursor_pointer()
            .hover(move |style| style.bg(wash))
            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            })
            .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
                chose(view, index, window, cx);
                cx.notify();
            }));
    }
    element.into_any_element()
}
