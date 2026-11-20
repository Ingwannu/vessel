//! The pane while its pull request is read: Paseo's
//! `pull-request-panel/pane-skeleton.tsx` and `activity-skeleton.tsx`
//! (the title and subtitle bars, the toolbar's two buttons, "Checks" over
//! three rows, three activity rows), pulsing with the kit's one phase.
//!
//! Difference: Paseo sizes its bars in percent of the pane; the kit's
//! shapes take points, so each bar is its percent of the panel's default
//! content width (440 less 32), the same at the default width.

use vessel_app_kit::theme::spacing;
use vessel_app_kit::skeleton::{Opacity, Rounding, Shape, pulse, shape};
use vessel_app_kit::ui::{AnyElement, Div, ParentElement as _, Styled as _, div, px, size};
use vessel_app_kit::{Tokens, Type, TypeExt as _};

use crate::shell::Shell;

/// Paseo's `PaneContentToolbar`, 36 high.
const TOOLBAR: f32 = 36.0;

/// A bar at `width` by `height` with Paseo's small corner.
fn bar(width: f32, height: f32, opacity: Opacity, tokens: &Tokens) -> Div {
    shape(
        Shape {
            size: size(px(width), px(height)),
            rounding: Rounding::Small,
        },
        opacity,
        tokens,
    )
}

/// A round shape `across` wide.
fn dot(across: f32, opacity: Opacity, tokens: &Tokens) -> Div {
    bar(across, across, opacity, tokens).rounded_full()
}

/// Paseo's skeleton, top to bottom.
pub(super) fn skeleton(shell: &Shell, tokens: &Tokens) -> AnyElement {
    let tokens = *tokens;
    let checks = shell.words.pulls().checks().to_owned();
    pulse("pr-pane-skeleton", move |opacity| {
        let neutral = tokens.neutral;
        let row = |opacity| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S2))
                .px(px(spacing::S3))
                .min_h(px(spacing::S8))
                .child(dot(14.0, opacity, &tokens))
                .child(bar(245.0, 12.0, opacity, &tokens))
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::S2))
                    .p(px(spacing::S4))
                    .child(bar(306.0, 16.0, opacity, &tokens))
                    .child(bar(163.0, 12.0, opacity, &tokens)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(spacing::S2))
                    .h(px(TOOLBAR))
                    .px(px(spacing::S3))
                    .bg(neutral.surface0)
                    .border_b_1()
                    .border_color(neutral.border)
                    .child(bar(96.0, 24.0, opacity, &tokens).rounded(px(spacing::S2)))
                    .child(bar(96.0, 24.0, opacity, &tokens).rounded(px(spacing::S2))),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::S1))
                    .py(px(spacing::S2))
                    .child(
                        div()
                            .px(px(spacing::S3))
                            .py(px(spacing::S2))
                            .type_(Type::Meta)
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(neutral.foreground_muted)
                            .child(checks.clone()),
                    )
                    .child(row(opacity))
                    .child(row(opacity))
                    .child(row(opacity)),
            )
            .child(activity_rows(opacity, &tokens))
    })
}

/// One activity row: the avatar, a wide line over a narrow one.
fn activity_row(opacity: Opacity, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .child(dot(20.0, opacity, tokens))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .gap(px(spacing::S1))
                .child(bar(272.0, 12.0, opacity, tokens))
                .child(bar(175.0, 10.0, opacity, tokens)),
        )
}

/// Paseo's `PrActivitySkeleton`: three rows, 12 apart.
fn activity_rows(opacity: Opacity, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .px(px(spacing::S3))
        .py(px(spacing::S2))
        .child(activity_row(opacity, tokens))
        .child(activity_row(opacity, tokens))
        .child(activity_row(opacity, tokens))
}

/// The Activity section's skeleton while its first read is out.
pub(super) fn activity(tokens: &Tokens) -> AnyElement {
    let tokens = *tokens;
    pulse("pr-pane-activity-skeleton", move |opacity| {
        activity_rows(opacity, &tokens)
    })
}
