//! Capture complete target scopes while drawing, before a book alias can rebind.
use super::Picker;
use crate::shell::Shell;
use vessel_app_kit::ComboRow;
use vessel_app_view::{Machine, history::Scope};

fn machine<'a>(shell: &'a Shell, scope: &Scope) -> Option<&'a Machine> {
    shell.fleet.online().find(|machine| {
        machine.connection_context().as_ref() == Some(&scope.context)
            && machine.name == scope.machine.as_str()
    })
}

/// A metadata precheck, never a substitute for `selected.call` or `DeliveryGuard`.
pub(super) fn current(shell: &Shell, scope: &Scope) -> bool {
    machine(shell, scope).is_some_and(|machine| {
        machine
            .agents
            .iter()
            .any(|agent| agent.harness == scope.harness && agent.available())
    })
}

pub(super) fn rows(shell: &Shell, which: Picker) -> Vec<(Option<Scope>, ComboRow)> {
    let selected = shell.areas.import.model.scope();
    match which {
        Picker::Machine => shell
            .fleet
            .online()
            .map(|machine| {
                let mut row = ComboRow::new(machine.name.clone());
                let scope = machine.connection_context().and_then(|context| {
                    let harness = machine
                        .agents
                        .iter()
                        .find(|agent| agent.available())?
                        .harness
                        .clone();
                    Some(Scope {
                        machine: context.machine().clone(),
                        context,
                        harness,
                        profile: None,
                        cwd: None,
                    })
                });
                row.disabled = scope.is_none();
                row.checked = selected.is_some_and(|selected| {
                    scope
                        .as_ref()
                        .is_some_and(|scope| scope.context == selected.context)
                });
                (scope, row)
            })
            .collect(),
        Picker::Agent => selected
            .and_then(|scope| machine(shell, scope))
            .into_iter()
            .flat_map(|machine| &machine.agents)
            .map(|agent| {
                let mut row = ComboRow::new(agent.name.clone());
                row.checked = selected.is_some_and(|scope| scope.harness == agent.harness);
                row.disabled = !agent.available();
                let scope = selected.filter(|_| !row.disabled).map(|scope| {
                    let mut scope = scope.clone();
                    scope.harness = agent.harness.clone();
                    scope.profile = None;
                    scope
                });
                (scope, row)
            })
            .collect(),
        Picker::Account => {
            let mut default = ComboRow::new(shell.words.history().default_account());
            default.checked = selected.is_some_and(|scope| scope.profile.is_none());
            default.disabled = selected.is_none();
            let scope = selected.map(|scope| {
                let mut scope = scope.clone();
                scope.profile = None;
                scope
            });
            let mut rows = vec![(scope, default)];
            if let Ok(profiles) = &shell.areas.import.profiles {
                rows.extend(
                    profiles
                        .iter()
                        .filter(|profile| {
                            selected.is_some_and(|scope| profile.harness == scope.harness)
                        })
                        .map(|profile| {
                            let mut row = ComboRow::new(
                                profile
                                    .label
                                    .clone()
                                    .unwrap_or_else(|| profile.name.to_string()),
                            );
                            row.checked = selected
                                .is_some_and(|scope| scope.profile.as_ref() == Some(&profile.name));
                            let scope = selected.map(|scope| {
                                let mut scope = scope.clone();
                                scope.profile = Some(profile.name.clone());
                                scope
                            });
                            (scope, row)
                        }),
                );
            }
            rows
        }
    }
}
