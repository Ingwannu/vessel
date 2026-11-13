//! Pieces the workspace's screens draw more than once, in Paseo's settings
//! look (`components/settings/headings/settings-section.tsx`,
//! `styles/settings.ts`; the spec's appendix P, C4): a section with a
//! small muted title and its action, a card of rows (each 16 in, a
//! hairline between), a badge, a banner. Drawn from the kit's tokens; kept
//! here until track E's settings frame brings its own, which these then
//! become.

use vessel_app_kit::theme::radius;
use vessel_app_kit::ui::{
    Div, IntoElement, ParentElement as _, SharedString, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};

/// A badge's tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tone {
    /// Neutral.
    Plain,
    /// Good (approved).
    Ok,
    /// Needs attention (not approved, folder missing).
    Warn,
    /// Failed.
    Danger,
}

/// A small label beside a row's title.
pub(super) fn badge(text: impl Into<SharedString>, tone: Tone, tokens: &Tokens) -> Div {
    let color = match tone {
        Tone::Plain => tokens.neutral.foreground_muted,
        Tone::Ok => tokens.st_ready,
        Tone::Warn => tokens.st_review,
        Tone::Danger => tokens.neutral.destructive,
    };
    div()
        .flex_none()
        .px(px(6.0))
        .py(px(1.0))
        .rounded(px(radius::R1))
        .border_1()
        .border_color(tokens.neutral.border)
        .type_(Type::Meta)
        .text_color(color)
        .child(text.into())
}

/// A section: its title (12, muted, 4 in) with an action at the right, 12
/// above what it holds; 24 under it.
pub(super) fn section(
    title: impl Into<SharedString>,
    action: Option<Div>,
    body: impl IntoElement,
    tokens: &Tokens,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .mb(px(24.0))
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(8.0))
                .ml(px(4.0))
                .child(
                    div()
                        .flex_1()
                        .type_(Type::Meta)
                        .text_color(tokens.neutral.foreground_muted)
                        .child(title.into()),
                )
                .children(action),
        )
        .child(body)
}

/// A card of rows: the first surface, 8 corners, a hairline edge.
pub(super) fn rows(tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(px(radius::R2))
        .border_1()
        .border_color(tokens.neutral.border)
        .bg(tokens.neutral.surface1)
        .overflow_hidden()
}

/// One row: its title (any element), the facts under it (12, muted, 4
/// below), its controls at the right; 16 in on every side, 44 tall at least
/// as a touch target ([`divided`] adds the hairline between rows).
pub(super) fn row(
    title: impl IntoElement,
    facts: &[String],
    controls: Option<Div>,
    tokens: &Tokens,
) -> Div {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(10.0))
        .min_h(px(44.0))
        .p(px(16.0))
        .child(
            div()
                .flex_1()
                .min_w(px(160.0))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(title)
                .when(!facts.is_empty(), |column| {
                    column.child(
                        div()
                            .type_(Type::Meta)
                            .text_color(tokens.neutral.foreground_muted)
                            .child(facts.join(" \u{b7} ")),
                    )
                }),
        )
        .children(controls.map(|controls| {
            controls
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(6.0))
        }))
}

/// `row` with the hairline above it that every row but a card's first has.
pub(super) fn divided(row: Div, index: usize, tokens: &Tokens) -> Div {
    row.when(index > 0, |row| {
        row.border_t_1().border_color(tokens.neutral.border)
    })
}

/// A row's title.
pub(super) fn title(text: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Label)
        .text_color(tokens.neutral.foreground)
        .child(text.into())
}

/// A fact's value at a row's right, in mono when it is a path or a name.
pub(super) fn fact(text: impl Into<SharedString>, mono: bool, tokens: &Tokens) -> Div {
    div()
        .type_(if mono { Type::Mono } else { Type::Meta })
        .text_color(tokens.neutral.foreground_muted)
        .child(text.into())
}

/// A line of text where a list would be (empty, loading, why not).
pub(super) fn empty(text: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.into())
}

/// A banner: a sentence that stays until dismissed, in the danger tone
/// when `error`.
pub(super) fn banner(
    text: impl Into<SharedString>,
    error: bool,
    action: Option<Div>,
    tokens: &Tokens,
) -> Div {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(10.0))
        .px(px(12.0))
        .py(px(10.0))
        .rounded(px(radius::R2))
        .border_1()
        .border_color(if error {
            tokens.neutral.destructive
        } else {
            tokens.neutral.border
        })
        .bg(tokens.neutral.surface1)
        .child(
            div()
                .flex_1()
                .min_w(px(160.0))
                .type_(Type::Body)
                .text_color(if error {
                    tokens.neutral.destructive
                } else {
                    tokens.neutral.foreground
                })
                .child(text.into()),
        )
        .children(action)
}

use vessel_app_kit::ui::FluentBuilder as _;
