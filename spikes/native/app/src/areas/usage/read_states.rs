//! Recovery actions keep their rendered connection/parent and never imply a fetch.
use crate::shell::Shell;
use vessel_app_kit::{
    ButtonKind, ButtonState, ControlSizing, Tokens, Type, TypeExt as _, button_with_kind_at,
    outline_button, settings,
    theme::spacing,
    ui::{
        AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
        SharedString, StatefulInteractiveElement as _, Styled as _, div, px,
    },
};
use vessel_app_view::usage::{InventoryRefresh, OptionsContext, Phase};
use vessel_proto::limits::ExternalLookup;

pub(super) fn context(shell: &Shell) -> Option<OptionsContext> {
    let state = &shell.areas.usage;
    Some(OptionsContext::new(
        state.connection.clone()?,
        state.modal.current(),
    ))
}
pub(super) fn current(shell: &Shell, receipt: &OptionsContext) -> bool {
    let state = &shell.areas.usage;
    receipt.matches(state.connection.as_ref(), state.modal.current())
        && shell.usage_connection_current(receipt.machine())
        && state.policy_pending.is_none()
        && state.source_pending.is_none()
}
pub(super) fn retry(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.usage;
    if !matches!(
        state.inventory.phase(),
        Phase::Failed { retained: None, .. }
    ) {
        return None;
    }
    let receipt = context(shell)?;
    let enabled = current(shell, &receipt);
    Some(
        outline_button(
            "usage-inventory-retry",
            shell.words.usage().retry(),
            enabled,
            tokens,
        )
        .when(enabled, |button| {
            button.on_click(cx.listener(move |shell, _, _, cx| {
                if current(shell, &receipt)
                    && matches!(
                        shell.areas.usage.inventory.phase(),
                        Phase::Failed { retained: None, .. }
                    )
                {
                    shell.usage_accounts(InventoryRefresh::Cached);
                    cx.notify();
                }
            }))
        })
        .into_any_element(),
    )
}
/// Paseo's `UsageMessage` (usage-section.tsx 94-110): a settings card with
/// one centred muted line, for off, empty and similar states.
pub(super) fn muted_card(text: impl Into<SharedString>, tokens: &Tokens) -> Div {
    settings::card(tokens).child(
        div()
            .w_full()
            .p(px(spacing::S4))
            .flex()
            .justify_center()
            .text_center()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(text.into()),
    )
}

/// Paseo's off state (usage-section.tsx 56-62): the muted card naming the
/// machine, then "Turn on". The three-store disclosure stays beside the
/// lookup switch in the options popover (ADR 0042), and the saved-account
/// caveat under the section title.
pub(super) fn off(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let text = shell.words.usage();
    let receipt = context(shell);
    let enabled = receipt
        .as_ref()
        .is_some_and(|receipt| current(shell, receipt));
    let host = shell
        .areas
        .usage
        .connection
        .as_ref()
        .map_or("", |connection| connection.machine().as_str());
    let line = text
        .off_on(host)
        .unwrap_or_else(|_| text.lookup_off().to_owned());
    let state = if enabled {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .child(muted_card(line, tokens))
        .child(
            button_with_kind_at(
                "usage-inventory-enable",
                text.turn_on(),
                ButtonKind::Secondary,
                state,
                ControlSizing::Pointer,
                tokens,
            )
            .when(enabled, |button| {
                button.on_click(cx.listener(move |shell, _, _, cx| {
                    if let Some(receipt) = &receipt
                        && current(shell, receipt)
                        && shell
                            .areas
                            .usage
                            .inventory
                            .phase()
                            .observation()
                            .and_then(vessel_app_view::usage::Inventory::policy)
                            == Some(ExternalLookup::Off)
                    {
                        shell.usage_set_policy(receipt.machine(), ExternalLookup::On);
                        cx.notify();
                    }
                }))
            }),
        )
}
