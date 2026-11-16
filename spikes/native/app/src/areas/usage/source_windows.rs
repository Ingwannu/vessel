//! Independent percentages, explicit source tone and optional forecasts share the approved kit bar.
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens,
    ui::{
        AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
        StatefulInteractiveElement as _, Styled as _,
    },
    usage_meter::{self, Appearance, Share},
};
use vessel_app_view::usage::{DisplayPercent, Pin, default_pins, reset_label};
use vessel_proto::{
    conversation::FiniteNumber,
    usage_reports::{SourceReportEntry, UsageWindow},
};
pub(super) fn render(
    shell: &Shell,
    entry: &SourceReportEntry,
    window: &UsageWindow,
    retained: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let state = &shell.areas.usage;
    let source = entry.source();
    let target = state
        .inventory
        .selection()
        .filter(|_| super::sidebar::visible(shell))
        .cloned()
        .zip(state.inventory.observation_ticket());
    let Some((machine, ticket)) = target else {
        return window_body(shell, window, retained, None, tokens).into_any_element();
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
    let body = window_body(shell, window, retained, Some(pinned), tokens);
    let receipt = super::read_states::context(shell);
    let label = format!("{} · {}", entry.source_label, window.label);
    let description = if pinned {
        shell.words.usage().unpin()
    } else {
        shell.words.usage().pin()
    };
    body.id(vessel_app_kit::ui::SharedString::from(format!(
        "usage-source-pin-{}-{}",
        entry.id(),
        window.id
    )))
    .role(gpui::Role::CheckBox)
    .aria_toggled(if pinned {
        gpui::Toggled::True
    } else {
        gpui::Toggled::False
    })
    .aria_label(label)
    .aria_description(description)
    .cursor_pointer()
    .on_click(cx.listener(move |shell, _, window, cx| {
        if let Some(receipt) = &receipt
            && super::read_states::current(shell, receipt)
        {
            shell.usage_toggle_pin(&machine, ticket, pin.clone(), window, cx);
        }
    }))
    .into_any_element()
}

fn window_body(
    shell: &Shell,
    window: &UsageWindow,
    retained: bool,
    pinned: Option<bool>,
    tokens: &Tokens,
) -> vessel_app_kit::ui::Div {
    let used = window.used.and_then(|value| FiniteNumber::new(value.get()));
    let remaining = window
        .remaining
        .and_then(|value| FiniteNumber::new(value.get()));
    let percent =
        DisplayPercent::from_reported(used, remaining, shell.prefs.settings.usage.display);
    let mut body = super::card::bar()
        .child(super::card::label_row(
            window.label.as_str().to_owned(),
            percent_label(shell, percent),
            pinned,
            tokens,
        ))
        .children(percent.map(|percent| {
            usage_meter::bar(
                Share::percent(percent.whole()),
                Appearance::Normal(tone(vessel_app_view::usage::source_window_tone(window))),
                tokens,
            )
        }));
    let now = chrono::Utc::now();
    match vessel_app_view::usage::source_window_timing(window, retained, now) {
        vessel_app_view::usage::SourceWindowTiming::Reset(at) => {
            if at.is_some() {
                body = body.child(super::card::reset_line(
                    reset_label(at, shell.areas.usage.offset, now, shell.words),
                    false,
                    tokens,
                ));
            }
        }
        // Paseo shows a runs-out warning in the reset line's place, in danger
        // ink; Vessel keeps the source's shortfall after it on the same line.
        vessel_app_view::usage::SourceWindowTiming::Risk { at, shortfall } => {
            let percent = format!("{}%", shortfall.get());
            let text = shell.words.usage();
            let risk = vessel_app_view::usage::source_risk_label(at, now, shell.words);
            let shortfall = text.shortfall(&percent).unwrap_or(percent);
            body = body.child(super::card::reset_line(
                format!("{risk} · {shortfall}"),
                true,
                tokens,
            ));
        }
    }
    body
}
pub(super) fn percent_label(shell: &Shell, percent: Option<DisplayPercent>) -> String {
    let text = shell.words.usage();
    percent.map_or_else(
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
    )
}
pub(super) const fn tone(tone: vessel_app_view::usage::Tone) -> usage_meter::Tone {
    match tone {
        vessel_app_view::usage::Tone::Default => usage_meter::Tone::Default,
        vessel_app_view::usage::Tone::Ok => usage_meter::Tone::Ok,
        vessel_app_view::usage::Tone::Warning => usage_meter::Tone::Warning,
        vessel_app_view::usage::Tone::Danger => usage_meter::Tone::Danger,
    }
}
