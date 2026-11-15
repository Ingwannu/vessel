//! The composer's suggestion list (Paseo's
//! `components/ui/autocomplete.tsx`, `Autocomplete`, value for value): the
//! agent's `/` commands or the project's `@` files above the composer, the
//! chosen row on `surface2`, and over the list, for a chosen command that
//! says what it does, a card with its name, its description and the input
//! it takes.
//!
//! [Decision Log]
//! - Purpose: the composer's `/` and `@` menus (track P's rows b and d) in
//!   Paseo's look.
//! - Constraint: the text keeps the keys while the list shows (the screen
//!   moves the choice through `field::composer_hooks`), so the list takes
//!   no focus; a click chooses a row without taking the keys from the text.
//! - Alternatives: the kit's combobox (`picker.rs`: a search row of its
//!   own, its own focus, rows of another height); the menu (`menu.rs`: a
//!   context menu's rows, no description or icon).
//! - Choice: a list of its own with Paseo's values: surface1 under a
//!   one-point `borderAccent` at radius 8 with the medium shadow; 4 above
//!   and below the rows; a row at least 36 high, 12 across and 8 down; a
//!   command's description on the label's line, 12 and muted, cut at one
//!   line; a file's 14-point icon in an 18-wide column 4 before its path;
//!   at most 220 high, scrolling, the chosen row kept in view; the loading,
//!   empty and failed states as one muted line 12 inside. A screen may give
//!   it less room (a `max_h` on its parent column, as Paseo's popover caps
//!   it to the room above the composer): the rows' frame shrinks to one row
//!   at least and the chosen command's card keeps its height.
//! - Cost: a second list look beside the combobox's, as Paseo has.
//!
//! Motion (the owner's word 39; Paseo's list has none): the chosen row's
//! fill comes in over the screen's [`Motion`] when the choice moves, and is
//! there at once under reduced motion (GPUI's animations end at once then).
//! The list's own opening and closing are the screen's (it knows when it
//! stops drawing the list).

use std::cell::Cell;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    Animation, AnimationExt as _, AnyElement, ClickEvent, Context, Div, ElementId, ScrollHandle,
    SharedString, Window, div, px,
};

use crate::assets::Glyph;
use crate::fonts::Type;
use crate::picker::glyph;
use crate::popup::{MENU_RADIUS, Surfaces, shadow_md};
use crate::theme::{Tokens, spacing};
use crate::widgets::TypeExt as _;

/// Paseo's `maxHeight` for the list.
const MAX_HEIGHT: f32 = 220.0;
/// Paseo's row `minHeight`.
const ROW_MIN: f32 = 36.0;
/// One full row, the list's padding and its borders (Paseo's `minHeight`
/// of the container, 36 + 8 + 2).
const LIST_MIN: f32 = 46.0;
/// The file icon's column (`itemLeading`).
const LEADING: f32 = 18.0;
/// The file icon (`size={14}`).
const ICON: f32 = 14.0;
/// The description's gap under a file's path (`marginTop: 2`).
const DESCRIPTION_GAP: f32 = 2.0;

/// What a row stands for, which decides its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A command: its name, then what it does on the same line.
    Command,
    /// A file: its icon, its path, then a detail and a description.
    File,
    /// A folder, drawn as a file with the folder icon.
    Directory,
}

/// One row (Paseo's `AutocompleteOption`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutocompleteRow {
    /// What it is called (`/review`, a path).
    pub label: SharedString,
    /// What it takes (a command's input hint), in the card; beside a file's
    /// path.
    pub detail: Option<SharedString>,
    /// What it does, in the agent's words.
    pub description: Option<SharedString>,
    /// Its shape.
    pub kind: RowKind,
}

/// What the list shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listing {
    /// The rows, in the order drawn (the best nearest the text, last).
    Rows(Vec<AutocompleteRow>),
    /// Nothing to list yet: the sentence that says so.
    Loading(SharedString),
    /// Nothing answers: the sentence that says so.
    Empty(SharedString),
    /// The list could not be had: the whole sentence.
    Failed(SharedString),
}

/// What the list keeps between frames: its scroll, and the row last
/// brought into view (so a row the person scrolled away from is not pulled
/// back on every frame, only when the choice or the list changes).
#[derive(Debug, Default)]
pub struct AutocompleteState {
    scroll: ScrollHandle,
    revealed: Cell<Option<(usize, usize)>>,
}

impl AutocompleteState {
    /// A list scrolled to its top.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// How a change in the list moves: its easing and length, from the app's
/// one set of motion tokens (the kit has none of its own, and cannot read
/// the view's).
#[derive(Debug, Clone, Copy)]
pub struct Motion {
    /// The curve, from 0 to 1.
    pub ease: fn(f32) -> f32,
    /// How long.
    pub length: Duration,
}

impl Motion {
    /// No motion: every change at once (examples, tests).
    pub const NONE: Self = Self {
        ease: linear,
        length: Duration::ZERO,
    };
}

const fn linear(progress: f32) -> f32 {
    progress
}

/// The list over the composer: the chosen command's card (when it has a
/// description), then the rows or the one line saying why there are none.
/// `on_choose` takes a clicked row's index.
#[expect(
    clippy::too_many_arguments,
    reason = "a list is drawn from its rows, its choice, its state, what a click does and how it moves"
)]
pub fn autocomplete<V: 'static>(
    id: impl Into<SharedString>,
    listing: &Listing,
    chosen: Option<usize>,
    state: &AutocompleteState,
    on_choose: fn(&mut V, usize, &mut Window, &mut Context<'_, V>),
    motion: Motion,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> Div {
    let id: SharedString = id.into();
    let rows = match listing {
        Listing::Rows(rows) if !rows.is_empty() => rows,
        // A screen says why there are no rows (`Listing::Empty`); an empty
        // list drawn anyway is the frame alone.
        Listing::Rows(_) => return container(tokens),
        Listing::Loading(words) | Listing::Empty(words) | Listing::Failed(words) => {
            return container(tokens).child(note(words.clone(), tokens));
        }
    };
    let chosen = chosen.filter(|index| *index < rows.len());
    if let Some(index) = chosen
        && state.revealed.get() != Some((index, rows.len()))
    {
        // Paseo keeps the active row in view when it changes, and the list
        // pinned to the text's end when it opens (its first choice is the
        // last row).
        state.scroll.scroll_to_item(index);
        if state.scroll.bounds_for_item(index).is_some() {
            state.revealed.set(Some((index, rows.len())));
        } else {
            // GPUI places the row from the last frame's layout: on the
            // list's first frame there is none, so one more frame (a list
            // nothing else redraws stayed at its top).
            cx.notify();
        }
    }
    let card = chosen
        .and_then(|index| rows.get(index))
        .filter(|row| row.kind == RowKind::Command)
        .and_then(|row| {
            row.description
                .as_ref()
                .map(|description| (row, description))
        })
        .map(|(row, description)| detail_card(row, description, tokens));
    let list = div()
        .id(ElementId::Name(id.clone()))
        .min_h_0()
        .overflow_y_scroll()
        .track_scroll(&state.scroll)
        .py(px(spacing::S1))
        .children(rows.iter().enumerate().map(|(index, row)| {
            let number = u64::try_from(index).unwrap_or(u64::MAX);
            let element = row_element(ElementId::NamedInteger(id.clone(), number), row, tokens)
                .on_click(cx.listener(move |view, _event: &ClickEvent, window, cx| {
                    on_choose(view, index, window, cx);
                }));
            if chosen != Some(index) {
                return element.into_any_element();
            }
            let fill = Surfaces::of(tokens).surface2;
            if motion.length.is_zero() {
                return element.bg(fill).into_any_element();
            }
            // A new id for each chosen row: the fill comes in again when
            // the choice moves (GPUI drops an animation's state when its
            // element is not drawn).
            element
                .with_animation(
                    ElementId::NamedInteger(format!("{id}-chosen").into(), number),
                    Animation::new(motion.length).with_easing(motion.ease),
                    move |row, shown| row.bg(fill.opacity(shown)),
                )
                .into_any_element()
        }));
    div()
        .flex()
        .flex_col()
        .min_h_0()
        .gap(px(spacing::S1))
        .children(card)
        .child(container(tokens).child(list))
}

/// The list's frame (Paseo's `container`).
fn container(tokens: &Tokens) -> Div {
    let surfaces = Surfaces::of(tokens);
    div()
        .occlude()
        .flex()
        .flex_col()
        .min_h(px(LIST_MIN))
        .max_h(px(MAX_HEIGHT))
        .bg(surfaces.surface1)
        .border_1()
        .border_color(surfaces.border_accent)
        .rounded(px(MENU_RADIUS))
        .overflow_hidden()
        .shadow(shadow_md(tokens))
}

/// The one line said when there are no rows (Paseo's `emptyItem`).
fn note(words: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .px(px(spacing::S3))
        .py(px(spacing::S3))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(words.into())
}

/// One row; its fill is `surface2` under the pointer (and when chosen: the
/// caller's, with its motion).
fn row_element(
    id: impl Into<ElementId>,
    row: &AutocompleteRow,
    tokens: &Tokens,
) -> gpui::Stateful<Div> {
    let neutral = tokens.neutral;
    let surfaces = Surfaces::of(tokens);
    let frame = div()
        .id(id.into())
        .flex()
        .flex_row()
        .items_center()
        .min_h(px(ROW_MIN))
        .px(px(spacing::S3))
        .py(px(spacing::S2))
        .cursor_pointer()
        .hover(move |style| style.bg(surfaces.surface2));
    let label = div()
        .type_(Type::Body)
        .text_color(neutral.foreground)
        .child(row.label.clone());
    match row.kind {
        RowKind::Command => frame.child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S2))
                .child(label.flex_shrink_0())
                .children(row.description.clone().map(|description| {
                    div()
                        .flex_1()
                        .min_w_0()
                        .type_(Type::Meta)
                        .text_color(neutral.foreground_muted)
                        .truncate()
                        .child(description)
                })),
        ),
        RowKind::File | RowKind::Directory => {
            let icon = match row.kind {
                RowKind::Directory => Glyph::Folder,
                RowKind::File | RowKind::Command => Glyph::File,
            };
            frame
                .child(
                    div()
                        .w(px(LEADING))
                        .mr(px(spacing::S1))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(glyph(icon, ICON, neutral.foreground_muted)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(spacing::S2))
                                .child(label.min_w_0())
                                .children(row.detail.clone().map(|detail| {
                                    div()
                                        .type_(Type::Meta)
                                        .text_color(neutral.foreground_muted)
                                        .child(detail)
                                })),
                        )
                        .children(row.description.clone().map(|description| {
                            div()
                                .mt(px(DESCRIPTION_GAP))
                                .type_(Type::Meta)
                                .text_color(neutral.foreground_muted)
                                .truncate()
                                .child(description)
                        })),
                )
        }
    }
}

/// The chosen command's card (Paseo's `detailCard`): its name, what it does
/// (two lines at most), what it takes.
fn detail_card(row: &AutocompleteRow, description: &SharedString, tokens: &Tokens) -> AnyElement {
    let neutral = tokens.neutral;
    let surfaces = Surfaces::of(tokens);
    div()
        .flex()
        .flex_col()
        // A screen that gives the list less room than it wants (the room
        // above its composer) gets a shorter, scrolling list: the card
        // keeps its height, its words are never cut.
        .flex_none()
        .overflow_hidden()
        .bg(surfaces.surface1)
        .border_1()
        .border_color(surfaces.border_accent)
        .rounded(px(MENU_RADIUS))
        .px(px(spacing::S3))
        .py(px(spacing::S3))
        .shadow(shadow_md(tokens))
        .child(
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .truncate()
                .child(row.label.clone()),
        )
        .child(
            div()
                .mt(px(spacing::S1))
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .line_clamp(2)
                .text_ellipsis()
                .child(description.clone()),
        )
        .children(row.detail.clone().map(|detail| {
            div()
                .mt(px(spacing::S1))
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .truncate()
                .child(detail)
        }))
        .into_any_element()
}
