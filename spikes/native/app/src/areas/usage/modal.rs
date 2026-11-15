//! One shared usage area supplies a dialog on wide windows and a sheet on compact.
use super::{
    accounts,
    rows::{Density, message},
};
use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div,
};
use vessel_app_kit::{
    ComboHandlers, ComboHeader, ComboRow, DialogContent, DialogSize, Side, Tokens, combobox,
    dialog, outline_button,
};
use vessel_app_view::{
    devices::Target,
    prefs::PreferenceChange,
    usage::{HostAvailability, HostChoice, UsageHost},
};
use vessel_proto::NodeId;
const HOST_LIST_WIDTH: f32 = 240.0;

impl Shell {
    /// Shared callback for the fixed footer icon and the optional summary row.
    pub(crate) fn open_usage(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.usage;
        if let Err(reason) = state.modal.show(&mut state.tickets) {
            state.policy_error = Some(reason);
            cx.notify();
            return;
        }
        self.areas.usage.dialog.focus(window, cx);
        self.settle_usage(window);
        cx.notify();
    }
    fn close_usage(
        &mut self,
        expected: vessel_app_view::devices::Ticket,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.usage;
        if !state.modal.dismiss(expected) {
            return;
        }
        self.close_usage_options_for_parent(expected, window, cx);
        let state = &mut self.areas.usage;
        state.host_list.close_and_restore(window, cx);
        state.dialog.dismissed(window, cx);
        cx.notify();
    }
    fn open_usage_hosts(
        &mut self,
        opening: vessel_app_view::devices::Ticket,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.usage;
        if state.modal.current() != Some(opening) {
            return;
        }
        state.host_list.set_anchor(state.host_anchor.get());
        state.host_list.open(None, window, cx);
        cx.notify();
    }
    fn choose_usage_host(
        &mut self,
        opening: vessel_app_view::devices::Ticket,
        node: NodeId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let hosts = hosts(self);
        let Some(node) = self.areas.usage.modal.choose_host(opening, node, &hosts) else {
            return;
        };
        self.prefer(&PreferenceChange::UsageHost(Some(node)), window, cx);
        self.areas.usage.host_list.close();
        cx.notify();
    }
}
pub(super) fn hosts(shell: &Shell) -> Vec<UsageHost> {
    shell
        .fleet
        .machines
        .iter()
        .filter_map(UsageHost::of_machine)
        .collect()
}
pub(super) fn selected(shell: &Shell) -> Option<NodeId> {
    let hosts = hosts(shell);
    let active = shell
        .areas
        .session
        .open
        .as_ref()
        .and_then(|open| {
            shell
                .fleet
                .machines
                .iter()
                .find(|machine| machine.name == open.machine)
        })
        .and_then(|machine| machine.node);
    HostChoice {
        picked: shell.prefs.settings.usage.host,
        active,
        hosts: &hosts,
    }
    .modal()
}
pub(super) fn target(shell: &Shell) -> Target {
    if !shell.loaded {
        return Target::Loading;
    }
    let Some(node) = selected(shell) else {
        return Target::NoMachines;
    };
    shell
        .fleet
        .machines
        .iter()
        .find(|machine| machine.node == Some(node) && machine.connection_context().is_some())
        .map_or(Target::NoMachines, |machine| {
            Target::Online(vessel_app_view::devices::MachineKey::from_book(
                &machine.name,
            ))
        })
}
pub(crate) fn dialogs(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.usage;
    let opening = state.modal.current()?;
    let text = shell.words.usage();
    let compact = vessel_app_kit::breakpoint::phone(window);
    let density = if compact {
        Density::Compact
    } else {
        Density::Wide
    };
    let connected: Vec<_> = hosts(shell)
        .into_iter()
        .filter(|host| host.availability == HostAvailability::Connected)
        .collect();
    let mut body = div().min_w_0().w_full().flex().flex_col();
    if connected.len() > 1 {
        let label = connected
            .iter()
            .find(|host| Some(host.node) == selected(shell))
            .map_or(text.host(), |host| host.label.as_str())
            .to_owned();
        body = body.child(
            outline_button("usage-host-filter", label, true, tokens)
                .child(vessel_app_kit::anchor_probe(&state.host_anchor))
                .on_click(cx.listener(move |shell, _, window, cx| {
                    shell.open_usage_hosts(opening, window, cx);
                })),
        );
    }
    body = match state.target {
        Target::Online(_) => body.child(accounts::render(shell, tokens, density, cx)),
        Target::Hidden | Target::Loading => body.child(message(text.loading(), tokens)),
        Target::NoMachines => body.child(message(text.no_hosts(), tokens)),
        Target::Offline(_) => body.child(message(text.offline(), tokens)),
    };
    body = body.children(super::options::trigger(
        shell,
        "usage-options-modal",
        tokens,
        compact,
        cx,
    ));
    let content = DialogContent {
        title: text.title().into(),
        body: body.into_any_element(),
        footer: Vec::new(),
        size: DialogSize::Wide,
    };
    Some(
        div()
            .child(dialog(
                &state.dialog,
                content,
                tokens,
                window,
                compact,
                move |shell: &mut Shell, window, cx| shell.close_usage(opening, window, cx),
                cx,
            ))
            .children(host_list(shell, &connected, opening, tokens, window, cx))
            .into_any_element(),
    )
}
fn host_list(
    shell: &Shell,
    hosts: &[UsageHost],
    opening: vessel_app_view::devices::Ticket,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let picked = selected(shell);
    let rows: Vec<_> = hosts
        .iter()
        .map(|host| {
            let mut row = ComboRow::new(host.label.clone());
            row.checked = Some(host.node) == picked;
            row
        })
        .collect();
    let identities: Vec<_> = hosts.iter().map(|host| host.node).collect();
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.usage.host_list,
        chose: Rc::new(move |shell: &mut Shell, index, window, cx| {
            if let Some(node) = identities.get(index) {
                shell.choose_usage_host(opening, *node, window, cx);
            }
        }),
        back: None,
    };
    let header = ComboHeader {
        title: None,
        back: false,
        search: false,
        empty: shell.words.usage().no_hosts().into(),
        note: None,
        side: Side::Below,
    };
    combobox(
        "usage-hosts",
        &shell.areas.usage.host_list,
        &header,
        &rows,
        &handlers,
        HOST_LIST_WIDTH,
        tokens,
        window,
        cx,
    )
}
