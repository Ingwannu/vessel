//! Pieces the workspace's screens draw more than once, in the page's
//! settings look (`settings-page.tsx`, `ui/kit/row.tsx`, `badge.tsx`,
//! `banner.tsx`): a section with its title and action, a row of facts, a
//! badge, a banner. Drawn from the kit's tokens; kept here until the kit
//! has its own (the coordinator decides whose they are).

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
        Tone::Plain => tokens.text_secondary,
        Tone::Ok => tokens.st_ready,
        Tone::Warn => tokens.st_review,
        Tone::Danger => tokens.danger,
    };
    div()
        .flex_none()
        .px(px(6.0))
        .py(px(1.0))
        .rounded(px(radius::R1))
        .border_1()
        .border_color(tokens.border_subtle)
        .type_(Type::Meta)
        .text_color(color)
        .child(text.into())
}

/// A page of sections, as Settings draws one: its lead, then the sections,
/// in a column no wider than a reading line, scrolled by the caller.
pub(super) fn page(lead: &'static str, tokens: &Tokens) -> Div {
    div()
        .w_full()
        .max_w(px(760.0))
        .flex()
        .flex_col()
        .gap(px(20.0))
        .px(px(16.0))
        .py(px(16.0))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.text_secondary)
                .child(lead),
        )
}

/// A section: its title, an action at the right, and what it holds.
pub(super) fn section(
    title: impl Into<SharedString>,
    action: Option<Div>,
    body: impl IntoElement,
    tokens: &Tokens,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(8.0))
                .min_h(px(32.0))
                .child(
                    div()
                        .flex_1()
                        .type_(Type::Label)
                        .text_color(tokens.text_primary)
                        .child(title.into()),
                )
                .children(action),
        )
        .child(body)
}

/// A list of rows on a card.
pub(super) fn rows(tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(px(radius::R4))
        .border_1()
        .border_color(tokens.border_subtle)
        .bg(tokens.bg_surface)
        .overflow_hidden()
}

/// One row: its title (any element), the facts under it, and its controls
/// at the right; 44 points tall at least, as a touch target.
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
        .px(px(12.0))
        .py(px(8.0))
        .border_b_1()
        .border_color(tokens.border_faint)
        .child(
            div()
                .flex_1()
                .min_w(px(160.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(title)
                .when(!facts.is_empty(), |column| {
                    column.child(
                        div()
                            .type_(Type::Meta)
                            .text_color(tokens.text_tertiary)
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

/// A row's title.
pub(super) fn title(text: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Label)
        .text_color(tokens.text_primary)
        .child(text.into())
}

/// A fact's value at a row's right, in mono when it is a path or a name.
pub(super) fn fact(text: impl Into<SharedString>, mono: bool, tokens: &Tokens) -> Div {
    div()
        .type_(if mono { Type::Mono } else { Type::Meta })
        .text_color(tokens.text_secondary)
        .child(text.into())
}

/// A line of text where a list would be (empty, loading, why not).
pub(super) fn empty(text: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Body)
        .text_color(tokens.text_tertiary)
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
            tokens.danger
        } else {
            tokens.border_subtle
        })
        .bg(tokens.bg_surface)
        .child(
            div()
                .flex_1()
                .min_w(px(160.0))
                .type_(Type::Body)
                .text_color(if error {
                    tokens.danger
                } else {
                    tokens.text_primary
                })
                .child(text.into()),
        )
        .children(action)
}

use vessel_app_kit::ui::FluentBuilder as _;
