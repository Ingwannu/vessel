//! Usage actions compose the shared typed button; geometry stays in the kit.
use crate::shell::Shell;
use vessel_app_kit::{
    ButtonState, ControlSizing, Tokens,
    assets::Glyph,
    toolbar_button::{ToolbarButton, ToolbarConfig, ToolbarIcon, ToolbarInk},
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
) -> ToolbarButton {
    let state = if enabled {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    let config = ToolbarConfig::new(ControlSizing::from_compact(compact), state);
    if busy {
        ToolbarButton::loading(id, label, ToolbarInk::ExtraMuted, config, tokens)
    } else {
        ToolbarButton::new(
            id,
            label,
            ToolbarIcon::new(Glyph::RotateCw, ToolbarInk::ExtraMuted).adaptive(),
            config,
            tokens,
        )
    }
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
    let button = refresh_button(
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
            shell.usage_refresh_all(receipt.machine());
            cx.notify();
        }
    }));
    Some(state.source_tips.action(button, id).into_any_element())
}
