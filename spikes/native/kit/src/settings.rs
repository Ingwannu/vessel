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
//! - Differences: the switch does not animate yet (Paseo's slides in 180
//!   ms); a focused switch shows nothing (the owner's rule: no ring).

use gpui::prelude::*;
use gpui::{AnyElement, BoxShadow, Div, ElementId, SharedString, Stateful, div, point, px, rgba};

use crate::fonts::Type;
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// The switch: on or off; `enabled` false fades it; `touch` sizes it for
/// a finger. The screen handles its click.
#[must_use]
pub fn switch(
    id: impl Into<ElementId>,
    on: bool,
    enabled: bool,
    touch: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let (width, height, thumb) = if touch {
        (44.0, 26.0, 22.0)
    } else {
        (34.0, 20.0, 16.0)
    };
    let (track, knob) = if on {
        (neutral.accent, neutral.accent_foreground)
    } else {
        (neutral.surface3, gpui::white())
    };
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .flex_shrink_0()
        .w(px(width))
        .h(px(height))
        .p(px(2.0))
        .rounded_full()
        .bg(track)
        .when(on, Styled::justify_end)
        .when(!enabled, |track| track.opacity(0.5))
        .when(enabled, Styled::cursor_pointer)
        .child(
            div()
                .size(px(thumb))
                .rounded_full()
                .bg(knob)
                .shadow(vec![BoxShadow {
                    color: rgba(0x0000_0040).into(),
                    offset: point(px(0.0), px(1.0)),
                    blur_radius: px(2.0),
                    spread_radius: px(0.0),
                    inset: false,
                }]),
        )
}

/// A settings section: its small title, then `card`.
#[must_use]
pub fn section(title: impl Into<SharedString>, card: Div, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_col()
        .mb(px(24.0))
        .child(
            div()
                .mb(px(12.0))
                .ml(px(4.0))
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(title.into()),
        )
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
    let neutral = tokens.neutral;
    let mut words = div().flex_1().min_w_0().flex().flex_col().child(
        div()
            .type_(Type::Body)
            .text_color(neutral.foreground)
            .child(title.into()),
    );
    if let Some(hint) = hint {
        words = words.child(
            div()
                .mt(px(4.0))
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(hint),
        );
    }
    if let Some(error) = error {
        words = words.child(
            div()
                .mt(px(4.0))
                .type_(Type::Meta)
                .text_color(neutral.destructive)
                .child(error),
        );
    }
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        // Paseo's row content keeps 12 from its control (`marginRight`).
        .gap(px(12.0))
        .p(px(16.0))
        .when(!first, |row| row.border_t_1().border_color(neutral.border))
        .child(words)
        .children(control)
}
