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

/// `text` on one line: its line breaks and runs of spaces as one space.
/// An agent's description can have several lines (Claude Code's subagents'
/// do); GPUI starts a new line at every break even where the text does not
/// wrap, so one line is made here, as a browser makes it for Paseo's
/// `numberOfLines={1}` (`white-space: nowrap` folds the breaks).
#[must_use]
pub fn one_line(text: &SharedString) -> SharedString {
    if text.contains(['\n', '\r', '\t']) || text.contains("  ") {
        text.split_whitespace().collect::<Vec<_>>().join(" ").into()
    } else {
        text.clone()
    }
}

/// How far from its trigger a list opens (Paseo's `offset(5)`).
const OFFSET: f32 = 5.0;

/// The same, upward.
const LIFT: f32 = -5.0;

/// How far a list keeps from the window's edges (Paseo's
/// `collisionPadding`).
const EDGE: f32 = 16.0;

/// The offset and the edge's margin, taken from the room on a side.
const BEYOND: f32 = -21.0;

/// A list is never shorter than this, even with little room on its side
/// (it then overlaps the window's edge margin rather than vanish).
const MIN_HEIGHT: f32 = 120.0;

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
    pill_parts(
        id,
        PillParts {
            leading,
            label: Some(label.into()),
            quiet,
            caret: true,
            ..PillParts::default()
        },
        open,
        touch,
        tokens,
    )
}

/// What a pill shows: Paseo's toolbar control drops its caret, then its
/// label, as the composer narrows (`composer/agent-controls/layout.ts`).
#[derive(Debug, Clone, Default)]
pub struct PillParts {
    /// The glyph before the label.
    pub leading: Option<Glyph>,
    /// A project's mark before the label, in place of a glyph (its
    /// initial on a tile: [`initial_tile`]).
    pub initial: Option<SharedString>,
    /// The label; `None` for a square of the glyph alone (Paseo's
    /// `toolbarIconOnly`).
    pub label: Option<SharedString>,
    /// Words after the label, quieter.
    pub quiet: Option<SharedString>,
    /// Words before the label, quieter (a feature's name before its
    /// value).
    pub prefix: Option<SharedString>,
    /// The glyph's and the label's colour, when not the muted one (an
    /// on/off feature that is on).
    pub tint: Option<Hsla>,
    /// A word after them in a colour of its own (the effort's level, in
    /// its colour at the top step); the foreground's when `None`.
    pub after: Option<(SharedString, Option<Hsla>)>,
    /// The caret after them.
    pub caret: bool,
}

/// A pill of `parts` (see [`pill`]): Paseo's toolbar control, 28 high (44
/// on touch), 8 inside, its glyph 16 (20 on touch); without a label a
/// square of the same height.
#[must_use]
pub fn pill_parts(
    id: impl Into<ElementId>,
    parts: PillParts,
    open: bool,
    touch: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let glyph_size = if touch { 20.0 } else { 16.0 };
    let height = if touch { 44.0 } else { 28.0 };
    let square = parts.label.is_none();
    // Paseo's toolbar control gives way when the row is short of room
    // (`minWidth: 0, flexShrink: 1`), its words cut with an ellipsis
    // (`numberOfLines={1}`); a square of a glyph alone does not
    // (`toolbarIconOnly`: `flexShrink: 0`).
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .h(px(height))
        .when(square, |pill| {
            pill.flex_shrink_0().w(px(height)).justify_center()
        })
        .when(!square, |pill| pill.min_w_0().flex_shrink(1.0).px(px(8.0)))
        .rounded(px(16.0))
        .when(open, |pill| pill.bg(neutral.surface0))
        .when(!open, |pill| {
            pill.hover(move |style| style.bg(neutral.surface2))
        })
        .cursor_pointer()
        .type_(Type::Body)
        .text_color(parts.tint.unwrap_or(neutral.foreground_muted))
        .children(parts.leading.map(|leading| {
            glyph(
                leading,
                glyph_size,
                parts.tint.unwrap_or(neutral.foreground_muted),
            )
            .flex_shrink_0()
        }))
        .children(
            parts
                .initial
                .map(|initial| initial_tile(initial, tokens).flex_shrink_0()),
        )
        .children(parts.prefix.map(|prefix| {
            div()
                .flex_shrink_0()
                .text_color(neutral.foreground_extra_muted)
                .child(prefix)
        }))
        .children(
            parts
                .label
                .map(|label| div().min_w_0().flex_shrink(1.0).truncate().child(label)),
        )
        .children(parts.quiet.map(|quiet| {
            div()
                .min_w_0()
                .flex_shrink(1.0)
                .truncate()
                .text_color(neutral.foreground_extra_muted)
                .child(quiet)
        }))
        .children(parts.after.map(|(word, colour)| {
            div()
                .flex_shrink_0()
                .text_color(colour.unwrap_or(neutral.foreground))
                .child(word)
        }))
        .when(parts.caret, |pill| {
            pill.child(glyph(Glyph::ChevronDown, 14.0, neutral.foreground_muted).flex_shrink_0())
        })
}

/// Where an element is drawn, recorded in `cell` every frame (an absolute
/// layer over its parent): the anchor of a list opened from it later
/// ([`ComboboxState::set_anchor`]), when several triggers share one list.
#[must_use]
pub fn anchor_probe(cell: &Rc<Cell<Option<Bounds<Pixels>>>>) -> AnyElement {
    let cell = Rc::clone(cell);
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
#[derive(Debug, Clone, PartialEq)]
pub enum Leading {
    /// A glyph, muted.
    Glyph(Glyph),
    /// A 14-point leading glyph, as Paseo’s branch-switcher renderOption.
    SmallGlyph(Glyph),
    /// A state's dot (a machine).
    Dot(Hsla),
    /// A project's mark: its initial on a tile ([`initial_tile`]).
    Initial(SharedString),
}

/// A project's mark as Paseo draws one without an icon of its own
/// (`ProjectIconView`'s fallback): the name's first letter, upper case,
/// on a 16 tile, 10 high and semibold, muted.
#[must_use]
pub fn initial_tile(initial: SharedString, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .size(px(16.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .bg(neutral.surface3)
        .text_size(px(10.0))
        .line_height(px(16.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(neutral.foreground_muted)
        .child(initial)
}

/// A name's initial for [`initial_tile`] (`?` for an empty name).
#[must_use]
pub fn initial_of(name: &str) -> SharedString {
    name.chars().next().map_or_else(
        || "?".into(),
        |first| first.to_uppercase().collect::<String>().into(),
    )
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
    /// A switch at its right, on or off (choosing the row flips it): an
    /// agent's on/off option at the top of the model list.
    pub switch: Option<bool>,
    /// A line under the label, muted, that may wrap (what an option is).
    pub note: Option<SharedString>,
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
            switch: None,
            note: None,
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
    /// Which side of its trigger the list opens on.
    pub side: Side,
}

/// Which side of its trigger a list opens on: Paseo's `desktopPlacement`,
/// chosen by the caller and never flipped on a wide window (Paseo's web
/// combobox has no `flip`): the composer's controls open above
/// (`top-start`), the controls under a card below (`bottom-start`, the
/// host and folder pickers of its new-workspace form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Side {
    /// Above the trigger, its left edge on the trigger's.
    #[default]
    Above,
    /// Below the trigger, its left edge on the trigger's.
    Below,
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
        anchor_probe(&self.anchor)
    }

    /// Open next from where another trigger was drawn (one list shared by
    /// several triggers: a pill per feature, a row of the features sheet).
    pub fn set_anchor(&self, bounds: Option<Bounds<Pixels>>) {
        self.anchor.set(bounds);
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
    let anchor = state.anchor.get();
    let room = room(anchor, header.side, window);
    panel = if compact {
        panel
            .w_full()
            .max_h(gpui::relative(0.9))
            .rounded_t(px(popup::SHEET_RADIUS))
            .pb(px(16.0))
    } else {
        panel
            .w(px(width))
            .max_h(px(room))
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
        // Paseo's `offset(5)` from the trigger, on the caller's side.
        let (at, corner, offset) = match (header.side, anchor) {
            // Not drawn yet (the first frame): near the window's middle.
            (_, None) => (point(px(8.0), px(240.0)), Anchor::BottomLeft, 0.0),
            (Side::Above, Some(bounds)) => (bounds.origin, Anchor::BottomLeft, LIFT),
            (Side::Below, Some(bounds)) => (bounds.bottom_left(), Anchor::TopLeft, OFFSET),
        };
        anchored()
            .position(at)
            .offset(point(px(0.0), px(offset)))
            .anchor(corner)
            .snap_to_window_with_margin(px(EDGE))
            .child(panel)
    };
    // Over a dialog too (`kit::dialog` is at 2): a feature's list opens
    // from a row of the features sheet.
    Some(deferred(element).with_priority(3).into_any_element())
}

/// Paseo's `size` middleware: a list is at most 400 tall and never taller
/// than the room on its side of `anchor`, less the offset and the 16 kept
/// from the window's edge (`collisionPadding`).
fn room(anchor: Option<Bounds<Pixels>>, side: Side, window: &Window) -> f32 {
    anchor.map_or(MAX_HEIGHT, |bounds| {
        let top = f32::from(bounds.top());
        let bottom = f32::from(bounds.bottom());
        let height = f32::from(window.viewport_size().height);
        // `mul_add`: the distance without an arithmetic operator.
        let room = match side {
            Side::Above => top.mul_add(1.0, BEYOND),
            Side::Below => bottom.mul_add(-1.0, height).mul_add(1.0, BEYOND),
        };
        room.clamp(MIN_HEIGHT, MAX_HEIGHT)
    })
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
    // The rows scroll inside the panel's height; the title and the search
    // row above them stay (`min_h_0` lets the list be shorter than its rows).
    let mut list = div()
        .id("combobox-rows")
        .flex_shrink(1.0)
        .min_h_0()
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

/// A row's words: its label, its detail on the same line, its note under
/// them.
fn row_words(row: &ComboRow, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    let label = div()
        .flex_shrink_0()
        .max_w_full()
        .truncate()
        .text_color(neutral.foreground)
        .child(one_line(&row.label));
    let mut words = div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .flex()
        .flex_row()
        .items_baseline()
        .gap(px(8.0))
        .child(label);
    if let Some(note) = &row.note {
        // The label, and under it what the option is, wrapping.
        words = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(words)
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(neutral.foreground_muted)
                    .whitespace_normal()
                    .child(note.clone()),
            );
    }
    if let Some(detail) = &row.detail {
        words = words.child(
            div()
                .flex_shrink(1.0)
                .min_w_0()
                .truncate()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(one_line(detail)),
        );
    }
    words
}

/// Existing picker rows keep their 16-point leading glyph.
const ROW_GLYPH: f32 = 16.0;
/// Paseo’s branch-switcher supplies its own 14-point leading slot.
const SMALL_ROW_GLYPH: f32 = 14.0;

/// What leads a row.
fn row_leading(leading: Leading, tokens: &Tokens) -> AnyElement {
    let neutral = tokens.neutral;
    match leading {
        Leading::Glyph(which) => {
            glyph(which, ROW_GLYPH, neutral.foreground_muted).into_any_element()
        }
        // BranchSwitcher's explicit 14-point slot must not resize every
        // existing 16-point picker glyph merely because this caller needs it.
        Leading::SmallGlyph(which) => {
            glyph(which, SMALL_ROW_GLYPH, neutral.foreground_muted).into_any_element()
        }
        Leading::Dot(color) => div()
            .size(px(14.0))
            .flex()
            .items_center()
            .justify_center()
            .child(div().size(px(6.0)).rounded_full().bg(color))
            .into_any_element(),
        Leading::Initial(initial) => initial_tile(initial, tokens).into_any_element(),
    }
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
    // Paseo's option (`ComboboxItem`): its label, then its description on
    // the same line, each one line (`numberOfLines={1}`); the description
    // gives way first and ends in an ellipsis, then the label does. A row is
    // one line high whatever an agent writes, so rows never run into each
    // other.
    // Paseo's label does not shrink (`flexShrink: 0`): the description
    // gives way first, all of it, before the label loses a letter. Here it
    // is also at most the row's width, so a name longer than the row ends
    // in an ellipsis. (A small shrink factor is not enough: any shrink at
    // all cuts the label, "Workspace acce…" beside a whole description.)
    let words = row_words(row, tokens);
    let mut element = div()
        .id(ElementId::NamedInteger(
            "combobox-row".into(),
            u64::try_from(index).unwrap_or(u64::MAX),
        ))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .flex_shrink_0()
        .min_h(px(if compact { 44.0 } else { 36.0 }))
        .px(px(12.0))
        .py(px(8.0))
        .when(row.disabled, |element| element.opacity(0.55))
        .when(highlighted && !row.disabled, |element| element.bg(wash))
        .children(
            row.leading
                .clone()
                .map(|leading| row_leading(leading, tokens)),
        )
        .child(words)
        // Paseo's trailing slot: the check, then the row's own words (a
        // value at most 45% of the row, one line).
        .when(row.checked, |element| {
            element.child(glyph(Glyph::Check, 16.0, neutral.foreground_muted))
        })
        .children(row.trailing.clone().map(|trailing| {
            div()
                .flex_shrink(1.0)
                .min_w_0()
                .max_w(gpui::relative(0.45))
                .truncate()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(trailing)
        }))
        .when(row.opens, |element| {
            element.child(glyph(Glyph::ChevronRight, 14.0, neutral.foreground_muted))
        })
        .children(row.switch.map(|on| {
            crate::settings::switch(
                ElementId::NamedInteger(
                    "combobox-switch".into(),
                    u64::try_from(index).unwrap_or(u64::MAX),
                ),
                on,
                !row.disabled,
                compact,
                0,
                tokens,
            )
        }));
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

#[cfg(test)]
mod tests {
    //! A list's rows hold what real agents send (the owner's capture of
    //! 0.0.0.0.5: Claude Code's `Agent` option, its values `ocx-*` with
    //! descriptions of several lines, ran into the rows under them): every
    //! row is one line high and starts where the last one ended.

    use std::cell::RefCell;

    use gpui::{Bounds, Context, IntoElement, Render, TestAppContext, Window};

    use super::*;
    use crate::theme::{Scheme, Tokens};

    /// A description as Claude Code sends one for a subagent.
    const LONG: &str = "Use this agent for wide read-only sweeps across many files.\n\
        It reads excerpts, not whole files, and reports locations.\n\n\
        Examples:\n- find every caller of a function\n- list the modules that read a setting";

    struct Probe {
        state: ComboboxState,
        rows: Vec<ComboRow>,
        seen: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    }

    impl Render for Probe {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
            let tokens = Tokens::of(Scheme::Dark);
            let handlers = ComboHandlers {
                state: |probe: &mut Self| &mut probe.state,
                chose: Rc::new(|_probe: &mut Self, _index, _window, _cx| {}),
                back: None,
            };
            let seen = Rc::clone(&self.seen);
            let mut list =
                div().w(px(360.0)).flex().flex_col().on_children_prepainted(
                    move |bounds, _window, _cx| *seen.borrow_mut() = bounds,
                );
            for (index, row) in self.rows.iter().enumerate() {
                list = list.child(combo_row(
                    index,
                    row,
                    &self.state,
                    &handlers,
                    false,
                    &tokens,
                    cx,
                ));
            }
            list
        }
    }

    fn rows() -> Vec<ComboRow> {
        let mut rows = Vec::new();
        let mut plain = ComboRow::new("Default");
        plain.checked = true;
        rows.push(plain);
        for (name, detail) in [
            ("ocx-glm-5-2", LONG),
            ("ocx-gpt-5-6-luna", LONG),
            (
                "an agent whose name is far longer than any list is wide, so it has to end early",
                "and a description after it",
            ),
            (
                "한국어 이름의 하위 에이전트",
                "설명도 여러 줄입니다.\n둘째 줄\n셋째 줄",
            ),
        ] {
            let mut row = ComboRow::new(name);
            row.detail = Some(detail.into());
            row.trailing = Some("Loading".into());
            rows.push(row);
        }
        rows
    }

    #[gpui::test]
    fn rows_of_long_descriptions_are_one_line_and_never_overlap(cx: &mut TestAppContext) {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let seen = Rc::clone(&seen);
            move |window, cx| Probe {
                state: ComboboxState::new(window, cx),
                rows: rows(),
                seen,
            }
        });
        cx.run_until_parked();
        let bounds = seen.borrow().clone();
        assert_eq!(bounds.len(), rows().len(), "every row was laid out");
        let first = bounds.first().copied().expect("a row");
        for (index, row) in bounds.iter().enumerate() {
            assert_eq!(
                row.size.height, first.size.height,
                "row {index} is taller than a one-line row: its text took more than one line"
            );
            if let Some(next) = bounds.get(index.saturating_add(1)) {
                assert!(
                    row.bottom() <= next.top(),
                    "row {index} runs into the row under it"
                );
            }
        }
    }

    #[test]
    fn breaks_fold_into_one_line() {
        assert_eq!(one_line(&"a\n\nb  c\td".into()).as_ref(), "a b c d");
        let plain: SharedString = "plain words".into();
        assert_eq!(one_line(&plain), plain);
    }
}

#[cfg(test)]
mod glyph_size_tests {
    use super::{ROW_GLYPH, SMALL_ROW_GLYPH};

    #[test]
    fn a_branch_switcher_does_not_resize_existing_picker_glyphs() {
        assert_eq!(ROW_GLYPH.to_bits(), 16.0_f32.to_bits());
        assert_eq!(SMALL_ROW_GLYPH.to_bits(), 14.0_f32.to_bits());
    }
}
