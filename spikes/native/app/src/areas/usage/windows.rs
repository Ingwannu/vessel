//! Every independent allowance gets one shared kit meter and its actual reset.
use crate::shell::Shell;
use vessel_app_kit::ui::{Div, ParentElement as _, Styled as _, div, px};
use vessel_app_kit::{
    Tokens, Type, TypeExt as _,
    theme::spacing,
    usage_meter::{self, Appearance, Share},
};
use vessel_app_view::usage::{DisplayPercent, reset_label, window_label};
use vessel_proto::{conversation::FiniteNumber, limits::LimitWindow};

pub(super) fn draw(shell: &Shell, window: &LimitWindow, tokens: &Tokens) -> Div {
    let text = shell.words.usage();
    let state = &shell.areas.usage;
    let used = window.used.and_then(|value| FiniteNumber::new(value.get()));
    let value = used.map(|used| DisplayPercent::reported(used, state.display));
    let label = value.map_or_else(
        || text.none().to_owned(),
        |percent| {
            let percent = percent.text();
            match state.display {
                vessel_app_view::usage::DisplayAs::Used => {
                    text.value_used(&percent).unwrap_or(percent)
                }
                vessel_app_view::usage::DisplayAs::Remaining => {
                    text.value_remaining(&percent).unwrap_or(percent)
                }
            }
        },
    );
    let tone = match vessel_app_view::usage::window_tone(used, None) {
        vessel_app_view::usage::Tone::Default => usage_meter::Tone::Default,
        vessel_app_view::usage::Tone::Ok => usage_meter::Tone::Ok,
        vessel_app_view::usage::Tone::Warning => usage_meter::Tone::Warning,
        vessel_app_view::usage::Tone::Danger => usage_meter::Tone::Danger,
    };
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .p(px(spacing::S3))
        .child(
            div()
                .flex()
                .flex_wrap()
                .justify_between()
                .gap(px(spacing::S2))
                .type_(Type::Body)
                .child(window_label(window, shell.words))
                .child(label),
        )
        .children(value.map(|value| {
            usage_meter::bar(
                Share::percent(value.whole()),
                Appearance::Normal(tone),
                tokens,
            )
        }))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(reset_label(
                    window.resets_at,
                    state.offset,
                    chrono::Utc::now(),
                    shell.words,
                )),
        )
}
