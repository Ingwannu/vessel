//! Recovery actions keep their rendered connection/parent and never imply a fetch.
use super::rows::message;
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens, outline_button,
    ui::{
        AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
        StatefulInteractiveElement as _, div,
    },
};
use vessel_app_view::usage::{InventoryRefresh, OptionsContext, Phase};
use vessel_proto::limits::ExternalLookup;

fn context(shell: &Shell) -> Option<OptionsContext> {
    let state = &shell.areas.usage;
    Some(OptionsContext::new(
        state.connection.clone()?,
        state.modal.current(),
    ))
}
fn current(shell: &Shell, receipt: &OptionsContext) -> bool {
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
    if !matches!(state.inventory.phase(), Phase::Failed { .. }) {
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
                    && matches!(shell.areas.usage.inventory.phase(), Phase::Failed { .. })
                {
                    shell.usage_accounts(InventoryRefresh::Cached);
                    cx.notify();
                }
            }))
        })
        .into_any_element(),
    )
}
pub(super) fn off(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let text = shell.words.usage();
    let receipt = context(shell);
    let enabled = receipt
        .as_ref()
        .is_some_and(|receipt| current(shell, receipt));
    div()
        .child(message(text.lookup_off(), tokens))
        .child(message(text.lookup_disclosure(), tokens))
        .child(
            outline_button(
                "usage-inventory-enable",
                text.enable_lookup(),
                enabled,
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
                        shell.usage_set_policy(receipt.machine().clone(), ExternalLookup::On);
                        cx.notify();
                    }
                }))
            }),
        )
}
