//! Every independent allowance gets one shared kit meter and its actual reset.
use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{
    Tokens, Type, TypeExt as _,
    theme::spacing,
    usage_meter::{self, Appearance, Share},
};
use vessel_app_view::usage::{
    Account, DisplayPercent, Pin, default_pins, reset_label, window_label,
};
use vessel_proto::{conversation::FiniteNumber, limits::LimitWindow};

pub(super) fn draw(
    shell: &Shell,
    window: &LimitWindow,
    account: Option<&Account>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let state = &shell.areas.usage;
    let body = meter(shell, window, tokens);
    let Some(account) = account.filter(|_| super::sidebar::visible(shell)) else {
        return body.into_any_element();
    };
    let Some(machine) = state.inventory.selection().cloned() else {
        return body.into_any_element();
    };
    let Some(source) = vessel_app_view::usage::saved_source(account.harness()) else {
        return body.into_any_element();
    };
    let Some(ticket) = state.inventory.observation_ticket() else {
        return body.into_any_element();
    };
    let pin = Pin {
        source,
        window: window.id.clone(),
    };
    let defaults = state
        .inventory
        .phase()
        .observation()
        .map(|inventory| default_pins(&super::sidebar::candidates(inventory, shell.words)))
        .unwrap_or_default();
    let selected = shell.prefs.settings.usage.pins.contains(&pin, &defaults);
    let id = vessel_app_kit::ui::SharedString::from(format!(
        "usage-pin-{}-{}-{}",
        account.harness(),
        account.profile(),
        window.id
    ));
    let receipt = super::read_states::context(shell);
    let label = format!(
        "{} · {}",
        vessel_app_view::agent_name(account.harness()),
        window_label(window, shell.words)
    );
    let description = if selected {
        shell.words.usage().unpin()
    } else {
        shell.words.usage().pin()
    };
    body.id(id)
        .role(gpui::Role::CheckBox)
        .aria_toggled(if selected {
            gpui::Toggled::True
        } else {
            gpui::Toggled::False
        })
        .aria_label(label)
        .aria_description(description)
        .cursor_pointer()
        .child(vessel_app_kit::glyph(
            vessel_app_kit::assets::Glyph::Pin,
            14.0,
            if selected {
                tokens.neutral.foreground
            } else {
                tokens.neutral.foreground_muted
            },
        ))
        .on_click(cx.listener(move |shell, _, window, cx| {
            if let Some(receipt) = &receipt
                && super::read_states::current(shell, receipt)
            {
                shell.usage_toggle_pin(&machine, ticket, pin.clone(), window, cx);
            }
        }))
        .into_any_element()
}

fn meter(shell: &Shell, window: &LimitWindow, tokens: &Tokens) -> vessel_app_kit::ui::Div {
    let state = &shell.areas.usage;
    let used = window.used.and_then(|value| FiniteNumber::new(value.get()));
    let value = used.map(|used| DisplayPercent::reported(used, shell.prefs.settings.usage.display));
    let label = super::source_windows::percent_label(shell, value);
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
