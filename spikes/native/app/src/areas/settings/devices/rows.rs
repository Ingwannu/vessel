//! Device rows and their read states, drawn through the settings kit.

use crate::shell::Shell;
use chrono::Utc;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, outline_button, settings};
use vessel_app_kit::{breakpoint::TOUCH_TARGET, theme::spacing};
use vessel_app_view::devices::{
    DeviceBadge, Listing, Revoking, Target, details, exact, paired_devices,
};

pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.devices();
    let state = &shell.areas.settings.devices;
    let (title, body) = match &state.page.target {
        Target::Hidden | Target::Loading => (words.paired.to_owned(), loading(shell, tokens)),
        Target::NoMachines => (words.paired.to_owned(), message(words.no_machines, tokens)),
        Target::Offline(machine) => (
            words.paired_with(machine.as_str()),
            message(words.offline, tokens),
        ),
        Target::Online(machine) => (
            words.paired_with(machine.as_str()),
            listed(shell, tokens, compact, cx),
        ),
    };
    div()
        .flex()
        .flex_col()
        .child(settings::section(
            title,
            settings::card(tokens).child(body),
            tokens,
        ))
        .when(matches!(state.page.target, Target::Online(_)), |body| {
            body.child(super::invitation::render(shell, tokens, compact, cx))
        })
        .children(state.tips.render(tokens))
        .into_any_element()
}

fn listed(shell: &Shell, tokens: &Tokens, compact: bool, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.devices();
    let state = &shell.areas.settings.devices;
    let mut body = div().flex().flex_col();
    match &state.page.listing {
        Listing::Empty | Listing::Loading(_) => return loading(shell, tokens),
        Listing::Failed(why) => {
            body = body.child(div().p(px(spacing::S4)).child(vessel_app_kit::alert::alert(
                vessel_app_kit::alert::Content {
                    title: Some(words.list_error_title.into()),
                    description: Some(div().child(words.list_problem(*why)).into_any_element()),
                    action: Some(refresh(words.retry, tokens, compact, cx)),
                },
                crate::badge::alert_variant(why.notice()),
                vessel_app_kit::alert::Size::Small,
                tokens,
            )));
        }
        Listing::Ready(devices) => {
            let active = paired_devices(devices);
            if active.is_empty() {
                body = body.child(message(words.empty, tokens));
            }
            for (index, device) in active.into_iter().enumerate() {
                body = body.child(device_row(shell, device, index == 0, tokens, compact, cx));
            }
            body = body.child(settings::row_at(
                words.list_controls,
                None,
                None,
                Some(refresh(words.refresh, tokens, compact, cx)),
                false,
                compact,
                tokens,
            ));
        }
    }
    match state.page.revoking {
        Revoking::Failed(why) => {
            body.child(div().p(px(spacing::S4)).child(vessel_app_kit::alert::alert(
                vessel_app_kit::alert::Content {
                    title: None,
                    description: Some(div().child(words.revoke_problem(why)).into_any_element()),
                    action: None,
                },
                crate::badge::alert_variant(why.notice()),
                vessel_app_kit::alert::Size::Small,
                tokens,
            )))
        }
        Revoking::Empty | Revoking::Confirm(_) | Revoking::Sending { .. } => body,
    }
}

fn refresh(
    label: &'static str,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    outline_button("devices-refresh", label, true, tokens)
        .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
        .on_click(cx.listener(|shell, _event, _window, cx| {
            shell.devices_list();
            cx.notify();
        }))
        .into_any_element()
}

pub(super) fn message(text: impl Into<gpui::SharedString>, tokens: &Tokens) -> Div {
    div()
        .p(px(spacing::S4))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.into())
}

fn loading(shell: &Shell, tokens: &Tokens) -> Div {
    let words = shell.t.lang.devices();
    div()
        .p(px(spacing::S4))
        .child(busy(
            "devices-listing",
            words.loading_badge,
            shell.areas.settings.devices.page.listing.variant(),
            tokens,
        ))
        .child(
            div()
                .mt(px(spacing::S1))
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.loading),
        )
}

fn device_row(
    shell: &Shell,
    device: &vessel_proto::DeviceInfo,
    first: bool,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.devices();
    let state = &shell.areas.settings.devices;
    let id = device.id;
    let pending = matches!(state.page.revoking, Revoking::Sending { .. });
    // Keep the row's geometry while its confirmation is open above it.
    let action = outline_button(
        format!("device-revoke-{id}"),
        words.revoke,
        !pending,
        tokens,
    )
    .when(!pending, |button| {
        button.on_click(cx.listener(move |shell, _event, window, cx| {
            shell.devices_choose_revoke(id, window, cx);
        }))
    });
    // A caller's identity is not yet supplied by this connection. Never
    // identify this device by its label; device.self will supply the fact.
    let badge = DeviceBadge::of(device, None).map(|badge| {
        vessel_app_kit::badge::badge(
            words.state(badge),
            crate::badge::variant(badge.variant()),
            vessel_app_kit::badge::Size::Small,
            None,
            tokens,
        )
    });
    let row = super::row::compose(
        device.label.clone(),
        details(shell.t.lang, device, Utc::now(), state.offset),
        badge,
        action,
        first,
        compact,
        tokens,
    )
    .id(format!("device-row-{id}"));
    let row = match state.places.get(&id) {
        Some(place) => state.tips.on(
            row,
            place,
            exact(shell.t.lang, device, state.offset),
            compact,
        ),
        None => row,
    };
    row.into_any_element()
}

/// One progress signal: a neutral badge with the kit's activity glyph.
pub(super) fn busy(
    id: &'static str,
    label: &'static str,
    variant: vessel_app_view::badge::Variant,
    tokens: &Tokens,
) -> Div {
    div().flex().child(vessel_app_kit::badge::badge(
        label,
        crate::badge::variant(variant),
        vessel_app_kit::badge::Size::Small,
        Some(vessel_app_kit::slider::spinner(
            id,
            spacing::S3,
            tokens.neutral.foreground_muted,
        )),
        tokens,
    ))
}
