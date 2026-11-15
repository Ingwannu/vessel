use super::*;
use vessel_app_kit::{
    ButtonConfig, ButtonContent, ButtonKind, ButtonSize, ButtonState, ControlSizing, button_control,
};
use vessel_app_link::agents::{Ask, CatalogueFetch};
use vessel_app_view::agent_status::{
    catalogue::Read,
    pages::{MachineKey, RemovalFreshness},
};
impl Shell {
    pub(in crate::areas::settings::agents) fn agent_catalogue(
        &mut self,
        shown: &MachineKey,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.agent_context(Some(shown)) {
            return;
        }
        let Some(node) = super::super::page(self)
            .filter(|page| page.pending().is_none())
            .and_then(|page| page.inventory())
            .map(|inventory| inventory.node)
        else {
            return;
        };
        self.agent_ask(
            Ask::Catalogue {
                node,
                fetch: CatalogueFetch::Refresh,
            },
            cx,
        );
    }
    pub(in crate::areas::settings::agents) fn agent_initial_catalogue(
        &mut self,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(machine) = self.areas.settings.agents.machine.clone() else {
            return;
        };
        let wanted = super::super::page(self).is_some_and(|page| {
            page.selected().is_none()
                && page.pending().is_none()
                && page.inventory().is_some()
                && matches!(page.catalogue(), Read::NotRequested)
        });
        if wanted {
            self.agent_catalogue(&machine, cx);
        }
    }
}
pub(super) fn head(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let scope = shell.areas.settings.agents.machine.clone();
    let busy = super::super::page(shell).is_none_or(|page| page.pending().is_some());
    button_control(
        "registry-refresh",
        ButtonContent::Label(shell.words.agent_catalog().refresh().into()),
        ButtonConfig::new(
            ButtonKind::Ghost,
            if busy {
                ButtonState::Disabled
            } else {
                ButtonState::Enabled
            },
            ControlSizing::from_compact(compact),
        ),
        tokens,
    )
    .on_click(cx.listener(move |shell, _, _, cx| {
        if let Some(scope) = &scope {
            shell.agent_catalogue(scope, cx);
        }
    }))
    .into_any_element()
}
pub(super) fn command(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let scope = shell.areas.settings.agents.machine.clone();
    let page = super::super::page(shell);
    let ready = page.is_some_and(|page| {
        page.pending().is_none() && page.removal_freshness() == RemovalFreshness::Current
    });
    button_control(
        "provider-add-command",
        ButtonContent::Label(shell.words.agent_catalog().command_add().into()),
        ButtonConfig::new(
            ButtonKind::Outline,
            if ready {
                ButtonState::Enabled
            } else {
                ButtonState::Disabled
            },
            ControlSizing::from_compact(compact),
        )
        .with_size(ButtonSize::Small),
        tokens,
    )
    .on_click(cx.listener(move |shell, _, window, cx| {
        if shell.agent_context(scope.as_ref()) {
            shell.agent_open_add(None, window, cx);
        }
    }))
    .flex_shrink_0()
    .into_any_element()
}
