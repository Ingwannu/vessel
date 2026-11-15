//! Independent percentages, explicit source tone and optional forecasts share the approved kit bar.
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens, Type, TypeExt as _,
    theme::spacing,
    ui::{
        AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
        Styled as _, div, px,
    },
    usage_meter::{self, Appearance, Share},
};
use vessel_app_view::usage::{DisplayPercent, Pin, default_pins, reset_label, timestamp};
use vessel_proto::{
    conversation::FiniteNumber,
    usage_reports::{SourceReportEntry, UsageWindow},
};
pub(super) fn render(
    shell: &Shell,
    entry: &SourceReportEntry,
    window: &UsageWindow,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let text = shell.words.usage();
    let state = &shell.areas.usage;
    let source = entry.source();
    let used = window.used.and_then(|value| FiniteNumber::new(value.get()));
    let remaining = window
        .remaining
        .and_then(|value| FiniteNumber::new(value.get()));
    let percent =
        DisplayPercent::from_reported(used, remaining, shell.prefs.settings.usage.display);
    let label = percent.map_or_else(
        || text.none().to_owned(),
        |percent| {
            let percent = percent.text();
            match shell.prefs.settings.usage.display {
                vessel_app_view::usage::DisplayAs::Used => {
                    text.value_used(&percent).unwrap_or(percent)
                }
                vessel_app_view::usage::DisplayAs::Remaining => {
                    text.value_remaining(&percent).unwrap_or(percent)
                }
            }
        },
    );
    let tone = match vessel_app_view::usage::source_window_tone(window) {
        vessel_app_view::usage::Tone::Default => usage_meter::Tone::Default,
        vessel_app_view::usage::Tone::Ok => usage_meter::Tone::Ok,
        vessel_app_view::usage::Tone::Warning => usage_meter::Tone::Warning,
        vessel_app_view::usage::Tone::Danger => usage_meter::Tone::Danger,
    };
    let mut body = div()
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
                .child(window.label.as_str().to_owned())
                .child(label),
        )
        .children(percent.map(|percent| {
            usage_meter::bar(
                Share::percent(percent.whole()),
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
        );
    if let Some(at) = window.runs_out_at {
        let at = timestamp(at, state.offset, shell.words);
        body = body.child(super::rows::message(
            text.runs_out_at(&at).unwrap_or(at),
            tokens,
        ));
    }
    if let Some(shortfall) = window.shortfall {
        let percent = format!("{}%", shortfall.get());
        body = body.child(super::rows::message(
            text.shortfall(&percent).unwrap_or(percent),
            tokens,
        ));
    }
    let Some(machine) = state
        .inventory
        .selection()
        .filter(|_| super::sidebar::visible(shell))
        .cloned()
    else {
        return body.into_any_element();
    };
    let pin = Pin {
        source: source.clone(),
        window: window.id.clone(),
    };
    let defaults = state
        .inventory
        .phase()
        .observation()
        .map(|inventory| default_pins(&super::sidebar::candidates(inventory, shell.words)))
        .unwrap_or_default();
    let pinned = shell.prefs.settings.usage.pins.contains(&pin, &defaults);
    body.id(vessel_app_kit::ui::SharedString::from(format!(
        "usage-source-pin-{}-{}",
        entry.id(),
        window.id
    )))
    .cursor_pointer()
    .child(vessel_app_kit::glyph(
        vessel_app_kit::assets::Glyph::Pin,
        14.0,
        if pinned {
            tokens.neutral.foreground
        } else {
            tokens.neutral.foreground_muted
        },
    ))
    .on_click(cx.listener(move |shell, _, window, cx| {
        shell.usage_toggle_pin(machine.clone(), pin.clone(), window, cx)
    }))
    .into_any_element()
}
