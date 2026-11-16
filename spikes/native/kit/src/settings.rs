//! Settings' furniture, as Paseo's (`components/ui/switch.tsx`,
//! `components/settings/headings`, `styles/settings.ts`; values in
//! `refs/notes/ui-paseo-goat-spec.md` A5 and C4): the switch, a section
//! with its small title, the card that holds its rows, and a row with its
//! title and hint at the left and its control at the right.
//!
//! [Decision Log]
//! - Purpose: the About screen was loose text, a button and a chip that
//!   said "켬" where a switch belongs (the owner, on 0.0.0.0.5). Every
//!   settings screen (About now, the settings track's sections later) is
//!   built from these, so they read as Paseo's.
//! - Choice: Paseo's values: the switch's track 34 by 20 (44 by 26 on
//!   touch), its thumb 16 (22) inset 2, `surface3` and white when off,
//!   the accent and the accent's foreground when on, a small shadow under
//!   the thumb, half-faded when disabled; a section's title 12 muted, 12
//!   above its card; the card `surface1`, 8 round, a hairline of
//!   `border`; a row 16 by 16, a hairline of `border` above every row
//!   but the first, its title 14 in the foreground, its hint 12 muted 4
//!   under it, 12 between the words and the control.
//! - Differences: the track's colour changes at once (Paseo's fades with
//!   the thumb's slide); a focused switch shows nothing (the owner's rule:
//!   no ring).

use gpui::prelude::*;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, AnyElement, App, BoxShadow, Div, ElementId, SharedString,
    Stateful, Window, div, ease_in_out, point, px, rgba,
};

use crate::fonts::Type;
use crate::theme::{Tokens, spacing};
use crate::widgets::TypeExt as _;

/// The switch: on or off; `enabled` false fades it; `touch` sizes it for
/// a finger; `turn` counts its changes (0: drawn in place; a new count
/// slides the thumb to its side in Paseo's 180 ms, `withTiming`'s
/// quadratic ease in and out). The screen handles its click.
#[must_use]
pub fn switch(
    id: impl Into<ElementId>,
    on: bool,
    enabled: bool,
    touch: bool,
    turn: u64,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let (width, height, thumb, travel) = if touch {
        (44.0, 26.0, 22.0, TOUCH_TRAVEL)
    } else {
        (34.0, 20.0, 16.0, POINTER_TRAVEL)
    };
    let (track, knob) = if on {
        (neutral.accent, neutral.accent_foreground)
    } else {
        (neutral.surface3, gpui::white())
    };
    let end = if on { travel } else { 0.0 };
    let knob = div()
        .absolute()
        .top(px(INSET))
        .left(px(end.mul_add(1.0, INSET)))
        .size(px(thumb))
        .rounded_full()
        .bg(knob)
        .shadow(vec![BoxShadow {
            color: rgba(0x0000_0040).into(),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(2.0),
            spread_radius: px(0.0),
            inset: false,
        }]);
    let knob = if turn == 0 {
        knob.into_any_element()
    } else {
        // From the other side to this one; a new `turn` starts it again.
        knob.with_animation(
            ElementId::NamedInteger("switch-turn".into(), turn),
            Animation::new(Duration::from_millis(MOTION_MS)).with_easing(ease_in_out),
            move |knob, delta| {
                let progress = if on { delta } else { delta.mul_add(-1.0, 1.0) };
                knob.left(px(progress.mul_add(travel, INSET)))
            },
        )
        .into_any_element()
    };
    div()
        .id(id)
        .relative()
        .flex_shrink_0()
        .w(px(width))
        .h(px(height))
        .rounded_full()
        .bg(track)
        .when(!enabled, |track| track.opacity(0.5))
        .when(enabled, Styled::cursor_pointer)
        .child(knob)
}

/// The thumb's inset in the track.
const INSET: f32 = 2.0;
/// How far the thumb travels: the track less the insets and the thumb.
const POINTER_TRAVEL: f32 = 14.0;
const TOUCH_TRAVEL: f32 = 18.0;
/// Paseo's switch moves in 180 ms.
const MOTION_MS: u64 = 180;

/// A settings section: its small title, then `card`.
#[must_use]
pub fn section(title: impl Into<SharedString>, card: Div, tokens: &Tokens) -> Div {
    section_with_trailing(title, card, None, tokens)
}

/// A settings section with an optional action at the end of its heading.
///
/// [Decision Log]
/// - Purpose: Refresh/Add belong beside the title, as in Paseo's
///   `components/settings/headings/settings-section.tsx`.
/// - Constraint: existing callers must retain their title/card geometry.
/// - Alternatives: each area draws another section head.
/// - Choice: one trailing slot; the existing section delegates with None.
/// - Why: the title's inset and its gap above the card stay shared.
/// - Cost: callers own their action's state, size and tooltip.
#[must_use]
pub fn section_with_trailing(
    title: impl Into<SharedString>,
    card: Div,
    trailing: Option<AnyElement>,
    tokens: &Tokens,
) -> Div {
    let mut head = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(spacing::S2))
        .mb(px(spacing::S3))
        .ml(px(spacing::S1))
        .child(
            div().flex().items_center().gap(px(spacing::S2)).child(
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(title.into()),
            ),
        );
    if let Some(trailing) = trailing {
        head = head.child(div().flex_none().child(trailing));
    }
    div()
        .flex()
        .flex_col()
        .mb(px(spacing::S6))
        .child(head)
        .child(card)
}

/// A section's card: its rows go in it, in order.
#[must_use]
pub fn card(tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .flex_col()
        .bg(neutral.surface1)
        .rounded(px(8.0))
        .border_1()
        .border_color(neutral.border)
        .overflow_hidden()
}

/// A row of a card: `title` and its `hint` at the left, `control` at the
/// right; `first` has no hairline above it.
#[must_use]
pub fn row(
    title: impl Into<SharedString>,
    hint: Option<SharedString>,
    control: Option<AnyElement>,
    first: bool,
    tokens: &Tokens,
) -> Div {
    row_with_error(title, hint, None, control, first, tokens)
}

/// [`row`] with an error under its hint (Paseo's `rowError`: 12 in the
/// danger colour, 4 under the hint).
#[must_use]
pub fn row_with_error(
    title: impl Into<SharedString>,
    hint: Option<SharedString>,
    error: Option<SharedString>,
    control: Option<AnyElement>,
    first: bool,
    tokens: &Tokens,
) -> Div {
    row_at(title, hint, error, control, first, false, tokens)
}

/// [`row_with_error`], laid out for its width: `compact` (a phone's), its
/// control moves under its words when both do not fit beside each other
/// (Paseo's `compactRow`: the words at least 160 wide, 12 between).
#[must_use]
pub fn row_at(
    title: impl Into<SharedString>,
    hint: Option<SharedString>,
    error: Option<SharedString>,
    control: Option<AnyElement>,
    first: bool,
    compact: bool,
    tokens: &Tokens,
) -> Div {
    row_faced(
        title,
        hint,
        Type::Meta,
        error,
        control,
        first,
        compact,
        tokens,
    )
}

/// A row whose words under the title are a value in the code face: a key's
/// fingerprint, an id (the strict brief: a hash or a key reads in the code
/// face, so each character is told from its neighbours).
#[must_use]
pub fn code_row(
    title: impl Into<SharedString>,
    value: impl Into<SharedString>,
    first: bool,
    tokens: &Tokens,
) -> Div {
    row_faced(
        title,
        Some(value.into()),
        Type::Mono,
        None,
        None,
        first,
        false,
        tokens,
    )
}

/// [`code_row`] whose value can be selected and copied: Paseo marks every
/// `DetailRow` value `selectable` (`hosts/host-confirmation-sheet.tsx`
/// 69-90), as a machine's id, a key's fingerprint or an endpoint.
///
/// [Decision Log]
/// - Purpose: a value a person may need to compare or paste elsewhere is
///   selected with the pointer, as Paseo's is; a copy button is not the
///   reference (the coordinator's verdict on the pairing offer, 2026-11-16).
/// - Existing asset: [`crate::plain_text`], the selectable literal text on
///   the shared text engine (selection, copy, no Markdown).
/// - Choice: the same row and words as [`code_row`] (title 14 in the
///   foreground, the value 12 in the code face, muted, 4 under it), with the
///   value drawn by `plain_text` under the caller's stable `id`; additive, so
///   `code_row` and its callers are unchanged.
/// - Cost: the value is one engine view per row, kept by `id` across frames;
///   it wraps inside the row's width and never widens the card.
#[must_use]
pub fn selectable_code_row(
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    value: &str,
    first: bool,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let value = hint_line(Type::Mono, tokens)
        .debug_selector(|| "settings-row-value".to_owned())
        .min_w_0()
        .child(crate::plain_text(id, value, 0, window, cx))
        .into_any_element();
    row_of_parts(
        title_of(title, tokens),
        Some(value),
        None,
        None,
        first,
        false,
        tokens,
    )
}

/// A row's title: 14 in the foreground.
fn title_of(title: impl Into<SharedString>, tokens: &Tokens) -> AnyElement {
    div()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground)
        .child(title.into())
        .into_any_element()
}

/// The line under a row's title: 4 below it, muted, in `face`.
fn hint_line(face: Type, tokens: &Tokens) -> Div {
    div()
        .mt(px(4.0))
        .type_(face)
        .text_color(tokens.neutral.foreground_muted)
}

#[expect(
    clippy::too_many_arguments,
    reason = "row_at's arguments and the face of its hint, one private body"
)]
fn row_faced(
    title: impl Into<SharedString>,
    hint: Option<SharedString>,
    hint_face: Type,
    error: Option<SharedString>,
    control: Option<AnyElement>,
    first: bool,
    compact: bool,
    tokens: &Tokens,
) -> Div {
    row_of(
        title_of(title, tokens),
        hint,
        hint_face,
        error,
        control,
        first,
        compact,
        tokens,
    )
}

/// [`row_at`] whose title is the caller's element: a title with something
/// beside it, as Paseo's rows put a `SettingsInfoTip` after their title.
/// The element sets its own face (the row's title is [`Type::Body`] in the
/// foreground).
#[must_use]
pub fn row_titled(
    title: AnyElement,
    hint: Option<SharedString>,
    error: Option<SharedString>,
    control: Option<AnyElement>,
    first: bool,
    compact: bool,
    tokens: &Tokens,
) -> Div {
    row_of(
        title,
        hint,
        Type::Meta,
        error,
        control,
        first,
        compact,
        tokens,
    )
}

/// The row's words (its title, then its hint and error under it) in the
/// shared frame.
#[expect(
    clippy::too_many_arguments,
    reason = "a row's parts, each optional, as its callers pass them"
)]
fn row_of(
    title: AnyElement,
    hint: Option<SharedString>,
    hint_face: Type,
    error: Option<SharedString>,
    control: Option<AnyElement>,
    first: bool,
    compact: bool,
    tokens: &Tokens,
) -> Div {
    let hint = hint.map(|hint| hint_line(hint_face, tokens).child(hint).into_any_element());
    row_of_parts(title, hint, error, control, first, compact, tokens)
}

/// The words (title, the line under it, an error) and the control, in the
/// shared row frame.
fn row_of_parts(
    title: AnyElement,
    hint: Option<AnyElement>,
    error: Option<SharedString>,
    control: Option<AnyElement>,
    first: bool,
    compact: bool,
    tokens: &Tokens,
) -> Div {
    let neutral = tokens.neutral;
    let words = div()
        .flex_1()
        .min_w_0()
        .when(compact, |words| words.flex_basis(px(160.0)))
        .flex()
        .flex_col()
        .child(title)
        .children(hint)
        .children(error.map(|error| {
            div()
                .mt(px(4.0))
                .type_(Type::Meta)
                .text_color(neutral.destructive)
                .child(error)
        }));
    row_with_content(words.into_any_element(), control, first, compact, tokens)
}

/// On a phone, a settings row's content box is the window's width less
/// this: the page's margin, the card's hairline and the row's padding, each
/// side (measured on a 390-point window: a 306-point content box, the card
/// at 27–366). A value given a set width there (a path cut to fit) uses it.
pub const PHONE_ROW_INSET: f32 = 84.0;

/// The shared settings row frame around custom content, with an optional
/// trailing control. The content owns its title/description arrangement.
///
/// [Decision Log]
/// - Purpose: Paseo provider rows combine marks, inline state and counts;
///   a string-only title cannot represent those contents.
/// - Constraint: ordinary setting rows keep the same frame and geometry.
/// - Alternatives: copy the padding/border into each area; widen `row_at`'s
///   title type and break callers accepting borrowed strings.
/// - Choice: `row_at` delegates to this frame; custom rows supply content.
/// - Why: styles/settings.ts's row is still implemented once.
/// - Cost: custom content owns its sizing, semantics and interaction.
#[must_use]
pub fn row_with_content(
    content: AnyElement,
    control: Option<AnyElement>,
    first: bool,
    compact: bool,
    tokens: &Tokens,
) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(spacing::S3))
        .when(compact, Styled::flex_wrap)
        .p(px(spacing::S4))
        .when(!first, |row| row.border_t_1().border_color(neutral.border))
        .child(content)
        .children(control)
}

#[cfg(test)]
mod selectable_tests;
#[cfg(test)]
mod tests;
