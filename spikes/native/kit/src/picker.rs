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

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Context, Div, ElementId, FocusHandle, Hsla, Pixels, SharedString,
    Stateful, Window, canvas, deferred, div, px, svg,
};

use crate::assets::Glyph;
use crate::field::TextField;
use crate::fonts::Type;
use crate::popup::Surfaces;
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

mod focus;
mod keys;
mod lifecycle;
mod pages;
mod panel;
mod rows;
mod send;
pub use send::{SendKind, send_button};

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
    /// An official embedded agent mark, replacing the generic leading glyph.
    pub agent_mark: Option<crate::agent_mark::Artwork>,
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
        .children(
            parts
                .leading
                .filter(|_| parts.agent_mark.is_none())
                .map(|leading| {
                    glyph(
                        leading,
                        glyph_size,
                        parts.tint.unwrap_or(neutral.foreground_muted),
                    )
                    .flex_shrink_0()
                }),
        )
        .children(parts.agent_mark.map(|art| {
            let size = if touch {
                crate::agent_mark::Size::Twenty
            } else {
                crate::agent_mark::Size::Sixteen
            };
            // The touch pill's existing 44-point height can hold the
            // OpenCode minimum plus quiet zone; the 28-point pill cannot.
            let space = if touch {
                crate::agent_mark::Space::Lockup
            } else {
                crate::agent_mark::Space::Compact
            };
            crate::agent_mark::agent_mark(Some(art), size, space, tokens)
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

/// What leads a row.
#[derive(Debug, Clone, PartialEq)]
pub enum Leading {
    /// A glyph, muted.
    Glyph(Glyph),
    /// A glyph in the foreground: a permissions mode's mark (Paseo's
    /// `ModeComboboxOption` draws its `DensityIcon` with `color="foreground"`).
    Mark(Glyph),
    /// A 14-point leading glyph, as Paseo’s branch-switcher renderOption.
    SmallGlyph(Glyph),
    /// An official agent mark, or the generic bot when unknown.
    AgentMark(Option<crate::agent_mark::Artwork>),
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
    /// The whole label, when `label` is a shortened form of it (a branch
    /// cut in its middle): shown in a code-face tooltip on hover, as the
    /// kit's tooltip shows one (nothing on touch).
    pub tooltip: Option<SharedString>,
    /// The page it opens beside the list while the list has submenus
    /// ([`ComboboxState::set_submenus`]): a press, Enter or the pointer at
    /// rest opens it there and the screen's `chose` is not called.
    pub page: Option<crate::submenu::PageId>,
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
            tooltip: None,
            page: None,
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
    /// A muted line above the rows, that may wrap: what to know about the
    /// list as a whole (it could not be refreshed, so it may be out of
    /// date). Paseo has none; `None` for none.
    pub note: Option<SharedString>,
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
    cycle: Rc<()>,
    return_to: crate::focus::ReturnFocus,
    highlighted: Option<usize>,
    /// The highlighted row's label: the highlight follows its row when rows
    /// are added or removed above it while the list is open (an agent's
    /// list refreshed behind it), rather than staying on an index.
    highlighted_label: Option<SharedString>,
    focus: FocusHandle,
    search: TextField,
    header_leading: Option<Leading>,
    /// Where its trigger was last drawn, in the window.
    anchor: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// Its rows' tooltips ([`ComboRow::tooltip`]) and where each row was
    /// drawn, kept across frames so a hovered row's tooltip finds it.
    tips: crate::Tooltips,
    tip_places: RowPlaces,
    /// Pages its rows open beside it ([`ComboboxState::set_submenus`]).
    submenus: Option<crate::submenu::Submenus>,
}

/// Each row's tooltip place, by row, the same one frame after frame (a
/// tooltip shown for a row compares its place by identity).
#[derive(Debug, Default)]
struct RowPlaces(RefCell<Vec<crate::tooltip::Place>>);

impl RowPlaces {
    /// Row `index`'s place, made the first time it is asked for.
    fn get(&self, index: usize) -> crate::tooltip::Place {
        let mut places = self.0.borrow_mut();
        while places.len() <= index {
            places.push(crate::tooltip::Place::default());
        }
        places.get(index).cloned().unwrap_or_default()
    }
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
            cycle: Rc::new(()),
            return_to: crate::focus::ReturnFocus::default(),
            highlighted: None,
            highlighted_label: None,
            focus: cx.focus_handle(),
            search,
            header_leading: None,
            anchor: Rc::new(Cell::new(None)),
            tips: crate::Tooltips::new(),
            tip_places: RowPlaces::default(),
            submenus: None,
        }
    }

    /// Rows with a [`ComboRow::page`] open it beside the list through
    /// `submenus` (a wide window; `None` keeps pages the screen's own), and a
    /// press on an open page is not outside the list.
    pub fn set_submenus(&mut self, submenus: Option<&crate::submenu::Submenus>) {
        self.submenus = submenus.cloned();
    }

    /// Replace the provider header's leading artwork as its list changes.
    /// Kept in control state so existing header literals and callers retain
    /// their no-artwork behaviour; no node identity enters the kit.
    pub fn set_header_leading(&mut self, leading: Option<Leading>) {
        self.header_leading = leading;
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
        let mut at = self.highlight_in(rows);
        for _ in 0..count {
            let next = match (at, forward) {
                (None, true) => 0,
                (None | Some(0), false) => count.saturating_sub(1),
                (Some(index), true) => index.saturating_add(1).checked_rem(count).unwrap_or(0),
                (Some(index), false) => index.saturating_sub(1),
            };
            at = Some(next);
            if let Some(row) = rows.get(next).filter(|row| !row.disabled) {
                self.highlighted = at;
                self.highlighted_label = Some(row.label.clone());
                return;
            }
        }
    }

    fn choice(&self, rows: &[ComboRow]) -> Option<usize> {
        // A query/probe can replace rows after the highlight was set.
        // Recheck availability at activation rather than trusting its index.
        self.highlight_in(rows)
            .filter(|index| rows.get(*index).is_some_and(|row| !row.disabled))
    }

    /// The highlighted row among `rows`: at its index while the row there
    /// is the one highlighted, else the row of that label nearest to it
    /// (rows were added or removed above it); none when it is gone.
    fn highlight_in(&self, rows: &[ComboRow]) -> Option<usize> {
        let index = self.highlighted?;
        let Some(label) = &self.highlighted_label else {
            return Some(index);
        };
        if rows.get(index).is_some_and(|row| &row.label == label) {
            return Some(index);
        }
        rows.iter()
            .enumerate()
            .filter(|(_, row)| &row.label == label)
            .min_by_key(|(at, _)| at.abs_diff(index))
            .map(|(at, _)| at)
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
    let list = panel::List {
        id: id.into(),
        state,
        header,
        rows,
        handlers,
        width,
    };
    let surface = if compact {
        panel::Surface::Sheet
    } else {
        panel::Surface::Popover
    };
    let drawn = panel::panel(&list, surface, tokens, window, cx);
    let placement = panel::Placement {
        compact,
        side: header.side,
        anchor: state.anchor.get(),
        lease: state.render_lease(),
    };
    let element = panel::placed(drawn, placement, handlers.state, window, cx);
    // Over a dialog too (`kit::dialog` is at 2): a feature's list opens
    // from a row of the features sheet.
    // A row's tooltip draws over the list (the tooltip's own priority).
    let tips = state.tips.render(tokens);
    Some(
        div()
            .child(deferred(element).with_priority(3))
            .children(tips)
            .into_any_element(),
    )
}

/// The list inside a flyout, a page beside its menu ([`crate::submenu`]):
/// the same panel, placed by [`crate::submenu::flyout`]; a press outside it
/// is its menu's to judge.
#[expect(
    clippy::too_many_arguments,
    reason = "a list is drawn from its state, its content, its handlers and the window's size"
)]
pub fn combobox_inline<V: 'static>(
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
    let list = panel::List {
        id: id.into(),
        state,
        header,
        rows,
        handlers,
        width,
    };
    let drawn = panel::panel(&list, panel::Surface::Flyout, tokens, window, cx);
    let tips = state.tips.render(tokens);
    Some(div().child(drawn).children(tips).into_any_element())
}

fn search_row<V: 'static>(state: &ComboboxState, tokens: &Tokens, cx: &mut Context<'_, V>) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(12.0))
        .py(px(12.0))
        .bg(Surfaces::of(tokens).surface1)
        .border_b_1()
        .border_color(tokens.neutral.border)
        .child(glyph(Glyph::Search, 14.0, tokens.neutral.foreground_muted))
        .child(div().flex_1().child(state.search.bare(tokens, cx)))
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

#[cfg(test)]
mod tests {
    //! A list's rows hold what real agents send (the owner's capture of
    //! 0.0.0.0.5: Claude Code's `Agent` option, its values `ocx-*` with
    //! descriptions of several lines, ran into the rows under them): every
    //! row is one line high and starts where the last one ended.

    use std::cell::RefCell;

    use gpui::{Bounds, Context, IntoElement, Render, TestAppContext, Window};

    use super::keys::key_handler;
    use super::rows::{Listing, combo_row, leading_column};
    use super::*;
    use crate::popup;
    use crate::theme::{Scheme, Tokens};

    #[test]
    fn a_row_keeps_its_tooltip_place_from_frame_to_frame() {
        let places = RowPlaces::default();
        let third = places.get(2);
        assert!(Rc::ptr_eq(&third, &places.get(2)));
        assert!(!Rc::ptr_eq(&third, &places.get(0)));
        assert!(Rc::ptr_eq(&places.get(0), &places.get(0)));
    }

    /// A description as Claude Code sends one for a subagent.
    const LONG: &str = "Use this agent for wide read-only sweeps across many files.\n\
        It reads excerpts, not whole files, and reports locations.\n\n\
        Examples:\n- find every caller of a function\n- list the modules that read a setting";

    struct Probe {
        state: ComboboxState,
        rows: Vec<ComboRow>,
        seen: Rc<RefCell<Vec<Bounds<Pixels>>>>,
        choices: Rc<RefCell<Vec<usize>>>,
    }

    impl Render for Probe {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
            let tokens = Tokens::of(Scheme::Dark);
            let handlers = ComboHandlers {
                state: |probe: &mut Self| &mut probe.state,
                chose: Rc::new(|probe: &mut Self, index, _window, _cx| {
                    probe.choices.borrow_mut().push(index);
                }),
                back: None,
            };
            let seen = Rc::clone(&self.seen);
            let mut list = div()
                .on_children_prepainted(move |bounds, _window, _cx| *seen.borrow_mut() = bounds)
                .id("picker-probe")
                .track_focus(&self.state.focus)
                .key_context(popup::KEY_CONTEXT)
                .capture_key_down(key_handler(&self.rows, &self.state, &handlers, cx))
                .w(px(360.0))
                .type_(Type::Body)
                .flex()
                .flex_col();
            for (index, row) in self.rows.iter().enumerate() {
                let listing = Listing {
                    column: leading_column(&self.rows),
                    highlighted: self.state.highlight_in(&self.rows),
                    state: &self.state,
                };
                list = list.child(combo_row(
                    index, row, listing, &handlers, false, &tokens, cx,
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
    fn disabled_marks_and_stale_highlights_never_call_the_selection_handler(
        cx: &mut TestAppContext,
    ) {
        let choices = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::new(RefCell::new(Vec::new()));
        let (probe, cx) = cx.add_window_view({
            let choices = Rc::clone(&choices);
            let seen = Rc::clone(&seen);
            move |window, cx| {
                let mut rows = vec![ComboRow::new("Available before")];
                for leading in [
                    Leading::AgentMark(Some(crate::agent_mark::GEMINI)),
                    Leading::AgentMark(None),
                    Leading::Glyph(Glyph::Bot),
                ] {
                    let mut row = ComboRow::new("Unavailable");
                    row.leading = Some(leading);
                    row.disabled = true;
                    rows.push(row);
                }
                rows.push(ComboRow::new("Available after"));
                let mut state = ComboboxState::new(window, cx);
                state.open(None, window, cx);
                Probe {
                    state,
                    rows,
                    seen,
                    choices,
                }
            }
        });
        cx.run_until_parked();
        for index in 1..=3 {
            let at = seen.borrow().get(index).unwrap().center();
            cx.simulate_click(at, gpui::Modifiers::default());
            cx.run_until_parked();
            assert!(choices.borrow().is_empty(), "disabled pointer row {index}");
        }
        cx.simulate_keystrokes("down down enter");
        assert_eq!(*choices.borrow(), [4]);
        cx.simulate_keystrokes("up enter");
        assert_eq!(*choices.borrow(), [4, 0]);
        assert!(cx.update(|_, cx| probe.read(cx).state.is_open()));
        for index in 1..=4 {
            cx.update(|_, cx| {
                probe.update(cx, |probe, cx| {
                    probe.rows[index].disabled = true;
                    // The highlight is an index and its row's label.
                    probe.state.highlighted = Some(index);
                    probe.state.highlighted_label = Some(probe.rows[index].label.clone());
                    cx.notify();
                });
            });
            cx.simulate_keystrokes("enter");
            assert_eq!(*choices.borrow(), [4, 0], "disabled Enter row {index}");
        }
        cx.update(|_, cx| {
            probe.update(cx, |probe, cx| {
                probe.rows.clear();
                cx.notify();
            });
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(*choices.borrow(), [4, 0], "removed/out-of-range row");
        cx.simulate_keystrokes("escape");
        assert!(!cx.update(|_, cx| probe.read(cx).state.is_open()));
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
                choices: Rc::default(),
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

    #[gpui::test]
    fn official_marks_keep_the_existing_row_height_at_both_ends(cx: &mut TestAppContext) {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let seen = Rc::clone(&seen);
            move |window, cx| {
                let mut rows: Vec<ComboRow> = [
                    Some(crate::agent_mark::OPENCODE),
                    Some(crate::agent_mark::CLAUDE),
                    None,
                    Some(crate::agent_mark::GEMINI),
                    Some(crate::agent_mark::CODEX),
                    Some(crate::agent_mark::COPILOT),
                    Some(crate::agent_mark::OPENCODE),
                ]
                .into_iter()
                .enumerate()
                .map(|(index, art)| {
                    let mut row = ComboRow::new("An agent with a long name / 긴 에이전트 이름");
                    row.leading = art.map(|art| Leading::AgentMark(Some(art)));
                    row.checked = index == 0;
                    row
                })
                .collect();
                for leading in [
                    Leading::AgentMark(None),
                    Leading::Glyph(Glyph::Folder),
                    Leading::Glyph(Glyph::Server),
                ] {
                    let mut row =
                        ComboRow::new("Generic and ordinary glyphs share their 16-point size");
                    row.leading = Some(leading);
                    rows.push(row);
                }
                Probe {
                    state: ComboboxState::new(window, cx),
                    rows,
                    seen,
                    choices: Rc::default(),
                }
            }
        });
        cx.run_until_parked();
        let bounds = seen.borrow();
        assert_eq!(bounds.len(), 10);
        assert_eq!(bounds.get(2).unwrap().size.height, px(37.0));
        for (index, row) in bounds.iter().enumerate() {
            assert_eq!(
                row.size.height,
                bounds.get(2).unwrap().size.height,
                "row {index} grew around the unmarked baseline row"
            );
            if let Some(next) = bounds.get(index.saturating_add(1)) {
                assert_eq!(
                    row.bottom(),
                    next.top(),
                    "mark rows must retain their existing adjacency"
                );
            }
        }
    }

    #[test]
    fn a_list_with_any_mark_keeps_the_mark_column_on_every_row() {
        let marked = |glyph: Glyph| {
            let mut row = ComboRow::new("marked");
            row.leading = Some(Leading::Mark(glyph));
            row
        };
        // Codex's modes: two marked, `read-only` without one.
        let modes = [
            ComboRow::new("Read Only"),
            marked(Glyph::Shield),
            marked(Glyph::ShieldOff),
        ];
        assert_eq!(leading_column(&modes), Some(16.0));
        // No row marked: no column, labels start at the row's edge.
        assert_eq!(
            leading_column(&[ComboRow::new("a"), ComboRow::new("b")]),
            None
        );
        // A machine's dot keeps its own width; the widest wins.
        let mut dot = ComboRow::new("machine");
        dot.leading = Some(Leading::Dot(gpui::black()));
        assert_eq!(
            leading_column(&[dot.clone(), ComboRow::new("x")]),
            Some(14.0)
        );
        assert_eq!(leading_column(&[dot, marked(Glyph::Shield)]), Some(16.0));
        // An agent list's rows carry their own marks, of the agent's width.
        let mut agent = ComboRow::new("agent");
        agent.leading = Some(Leading::AgentMark(None));
        assert_eq!(leading_column(&[agent, ComboRow::new("x")]), None);
    }

    #[gpui::test]
    fn the_highlight_follows_its_row_when_rows_are_added_above_it(cx: &mut TestAppContext) {
        let (probe, cx) = cx.add_window_view(|window, cx| Probe {
            state: ComboboxState::new(window, cx),
            rows: rows(),
            seen: Rc::default(),
            choices: Rc::default(),
        });
        cx.update(|_, cx| {
            probe.update(cx, |probe, _cx| {
                let rows = probe.rows.clone();
                probe.state.step(&rows, true);
                probe.state.step(&rows, true);
                assert_eq!(probe.state.highlight_in(&rows), Some(1));
                // A new row arrives above, as an agent's refreshed list
                // lays a new value at the edge away from the anchor.
                let mut added = vec![ComboRow::new("new above")];
                added.extend(rows.iter().cloned());
                assert_eq!(probe.state.highlight_in(&added), Some(2), "followed");
                assert_eq!(probe.state.choice(&added), Some(2), "Enter takes it");
                // Its row gone: no highlight rather than another row.
                let gone: Vec<ComboRow> = rows
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != 1)
                    .map(|(_, row)| row.clone())
                    .collect();
                assert_eq!(probe.state.highlight_in(&gone), None);
            });
        });
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
    use super::rows::{ROW_GLYPH, SMALL_ROW_GLYPH};

    #[test]
    fn a_branch_switcher_does_not_resize_existing_picker_glyphs() {
        assert_eq!(ROW_GLYPH.to_bits(), 16.0_f32.to_bits());
        assert_eq!(SMALL_ROW_GLYPH.to_bits(), 14.0_f32.to_bits());
    }
}
