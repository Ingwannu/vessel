//! The history host selector reuses the kit's checked combobox and compact sheet.
use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, StatefulInteractiveElement as _, Window,
};
use vessel_app_kit::{ComboHandlers, ComboRow, ComboboxState, Tokens, combobox, select};
use vessel_app_view::sessions::{HistoryPage, HostScope};

pub(super) struct Filters {
    list: ComboboxState,
    anchor: select::Anchor,
    choices: Vec<HostScope>,
}
impl Filters {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            list: ComboboxState::new(window, cx),
            anchor: select::Anchor::default(),
            choices: Vec::new(),
        }
    }
}
pub(super) fn trigger(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if shell.fleet.machines.len() < 2 {
        return None;
    }
    let area = &shell.areas.sessions;
    let label = match &area.scope {
        HostScope::All => shell.words.sessions().all_hosts(),
        HostScope::Host(name) => name.as_str(),
    };
    Some(
        select::trigger(
            "sessions-host",
            label.to_owned(),
            &area.filters.anchor,
            area.filters.list.is_open(),
            true,
            touch,
            tokens,
        )
        .on_click(cx.listener(|shell, _, window, cx| {
            let mut choices = vec![HostScope::All];
            for machine in &shell.fleet.machines {
                if let Ok(name) = vessel_app_view::machine_book::Name::parse(&machine.name) {
                    choices.push(HostScope::Host(name));
                } else {
                    shell.notice = Some(shell.words.sessions().navigation_refused().to_owned());
                    return;
                }
            }
            let state = &mut shell.areas.sessions.filters;
            state.choices = choices;
            state.list.set_anchor(state.anchor.get());
            state.list.open(None, window, cx);
            cx.notify();
        }))
        .into_any_element(),
    )
}
pub(super) fn picker(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.sessions;
    let rows: Vec<ComboRow> = area
        .filters
        .choices
        .iter()
        .map(|scope| {
            let label = match scope {
                HostScope::All => shell.words.sessions().all_hosts(),
                HostScope::Host(name) => name.as_str(),
            };
            ComboRow {
                checked: *scope == area.scope,
                ..ComboRow::new(label.to_owned())
            }
        })
        .collect();
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.sessions.filters.list,
        chose: Rc::new(|shell: &mut Shell, at, _, cx| {
            let area = &mut shell.areas.sessions;
            if let Some(scope) = area.filters.choices.get(at) {
                area.scope = scope.clone();
            }
            area.page = HistoryPage::default();
            area.filters.list.close();
            cx.notify();
        }),
        back: None,
    };
    combobox(
        "sessions-host-list",
        &area.filters.list,
        &select::header(),
        &rows,
        &handlers,
        select::WIDTH,
        tokens,
        window,
        cx,
    )
}
