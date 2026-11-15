//! Usage actions compose the shared typed button; geometry stays in the kit.
use crate::shell::Shell;
use vessel_app_kit::{
    Button, ButtonConfig, ButtonContent, ButtonIcon, ButtonKind, ButtonState, ControlSizing,
    Tokens,
    assets::Glyph,
    button_control,
    ui::{AnyElement, Context, ElementId, IntoElement as _, SharedString},
};
use vessel_app_view::usage::{Inventory, Phase};
use vessel_proto::{limits::ExternalLookup, usage_reports::UsageSourceAvailability};

pub(super) fn refresh_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    enabled: bool,
    busy: bool,
    compact: bool,
    tokens: &Tokens,
) -> Button {
    let state = if enabled {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    let config = ButtonConfig::new(
        ButtonKind::Ghost,
        state,
        ControlSizing::from_compact(compact),
    );
    let config = if busy { config.loading() } else { config };
    button_control(
        id,
        ButtonContent::Icon(ButtonIcon::muted(Glyph::RotateCw)),
        config,
        tokens,
    )
    .aria_label(label)
}
pub(super) fn refresh_all(
    shell: &Shell,
    id: &'static str,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.usage;
    let inventory = state.inventory.phase().observation();
    if inventory.and_then(Inventory::policy) == Some(ExternalLookup::Off) {
        return None;
    }
    let supported = match inventory {
        Some(Inventory::Sources { reports, .. }) => reports
            .sources
            .as_slice()
            .iter()
            .any(|source| source.availability == UsageSourceAvailability::Enabled),
        Some(Inventory::Saved { accounts, .. }) => {
            accounts.iter().any(|report| report.result.is_ok())
        }
        None => true,
    };
    if !supported {
        return None;
    }
    let receipt = super::read_states::context(shell)?;
    let busy = matches!(state.inventory.phase(), Phase::Idle | Phase::Loading { .. });
    let enabled = super::read_states::current(shell, &receipt) && !busy;
    Some(
        refresh_button(
            id,
            shell.words.usage().refresh_all(),
            enabled,
            busy,
            compact,
            tokens,
        )
        .on_click(cx.listener(move |shell, _, _, cx| {
            if super::read_states::current(shell, &receipt)
                && !matches!(shell.areas.usage.inventory.phase(), Phase::Loading { .. })
            {
                shell.usage_refresh_all(receipt.machine().clone());
                cx.notify();
            }
        }))
        .into_any_element(),
    )
}
