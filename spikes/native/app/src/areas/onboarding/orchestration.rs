//! Deferred integration choices keep their visit and batch through actual owner acknowledgements.

use super::{Context, Shell, Visit, Window};
use std::collections::BTreeMap;
use vessel_app_kit::ui::{
    AnyElement, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div,
};
use vessel_app_kit::{Tokens, outline_button, settings};
use vessel_app_view::desktop::cli::{
    Availability, Operation as CliOperation, Snapshot, State as CliState, Status as CliStatus,
};
use vessel_app_view::devices::{Ticket, Tickets};
use vessel_app_view::onboarding::orchestration::{
    Choice, Integration, Operation, Orchestration, Outcome, Settlement,
};

#[derive(Default)]
enum Facts {
    #[default]
    Loading,
    Ready(Snapshot),
    #[cfg(not(target_arch = "wasm32"))]
    Failed(vessel_app_link::desktop::DesktopError),
    #[cfg(target_arch = "wasm32")]
    Unavailable,
}

#[derive(Default)]
pub(super) struct Setup {
    model: Orchestration,
    // Abandoning a visit invalidates advancement, but does not cancel a CLI
    // write already authorized on Continue. Wait for it before another batch.
    running: Vec<Operation>,
    facts: Facts,
    tickets: Tickets,
    query: Option<Ticket>,
}

pub(super) fn opened(shell: &mut Shell, cx: &mut Context<'_, Shell>) {
    shell.areas.onboarding.integrations.model = Orchestration::default();
    refresh(shell, cx);
}

fn refresh(shell: &mut Shell, cx: &mut Context<'_, Shell>) {
    if !shell.areas.onboarding.integrations.running.is_empty() {
        return;
    }
    shell.areas.onboarding.integrations.facts = Facts::Loading;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let read = crate::areas::desktop::cli::snapshot(cx);
        let Ok(ticket) = shell.areas.onboarding.integrations.tickets.next_ticket() else {
            return;
        };
        shell.areas.onboarding.integrations.query = Some(ticket);
        cx.spawn(async move |this, cx| {
            let result = read.await;
            let _closed = this.update(cx, |shell, cx| {
                let facts = match result {
                    Ok(snapshot) => Facts::Ready(snapshot),
                    Err(error) => Facts::Failed(error),
                };
                let _accepted =
                    install_query(&mut shell.areas.onboarding.integrations, ticket, facts);
                cx.notify();
            });
        })
        .detach();
    }
    #[cfg(target_arch = "wasm32")]
    {
        shell.areas.onboarding.integrations.facts = Facts::Unavailable;
        cx.notify();
    }
}

pub(super) fn closed(shell: &mut Shell) {
    shell.areas.onboarding.integrations.query = None;
}

pub(super) fn leave(shell: &mut Shell) {
    shell.areas.onboarding.integrations.model.abandon();
}

pub(super) fn busy(shell: &Shell) -> bool {
    let setup = &shell.areas.onboarding.integrations;
    !setup.running.is_empty() || matches!(setup.facts, Facts::Loading)
}

pub(super) fn apply(
    shell: &mut Shell,
    visit: Visit,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if busy(shell) || !super::effects_allowed(cx) {
        return;
    }
    let mut offered = BTreeMap::new();
    if let Facts::Ready(snapshot) = shell.areas.onboarding.integrations.facts
        && let Some(current) = cli_choice(snapshot)
    {
        let _old = offered.insert(Integration::Cli, Some(current));
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Facts::Failed(error) = &shell.areas.onboarding.integrations.facts {
        tracing::warn!(%error,"setup could not read CLI availability");
        shell.notice = Some(shell.words.desktop().setup_failed().to_owned());
        return;
    }
    let Ok(operations) = shell
        .areas
        .onboarding
        .integrations
        .model
        .begin(visit, &offered)
    else {
        return;
    };
    if operations.is_empty() {
        shell.advance_setup(visit, window, cx);
        return;
    }
    for operation in operations {
        shell.areas.onboarding.integrations.running.push(operation);
        execute(shell, operation, window, cx);
    }
    cx.notify();
}

#[cfg(not(target_arch = "wasm32"))]
fn execute(
    shell: &mut Shell,
    operation: Operation,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    match operation.integration {
        Integration::Cli => {
            shell.areas.onboarding.integrations.query = None;
            let work = crate::areas::desktop::cli::set_installed(
                match operation.choice {
                    Choice::On => CliOperation::Install,
                    Choice::Off => CliOperation::Remove,
                },
                cx,
            );
            cx.spawn_in(window, async move |this, cx| {
                let result = work.await;
                let _closed = this.update_in(cx, |shell, window, cx| {
                    let outcome = match result {
                        Ok(snapshot) => {
                            if snapshot.status == CliStatus::Installed
                                && snapshot.path
                                    == vessel_app_view::desktop::cli::PathStatus::Manual
                            {
                                shell.notice =
                                    Some(shell.words.desktop_shell().path_manual().to_owned());
                            }
                            install_mutation(
                                &mut shell.areas.onboarding.integrations,
                                Facts::Ready(snapshot),
                            );
                            Outcome::Applied
                        }
                        Err(error) => {
                            install_mutation(
                                &mut shell.areas.onboarding.integrations,
                                Facts::Failed(error),
                            );
                            Outcome::Refused
                        }
                    };
                    settled(shell, operation, outcome, window, cx);
                });
            })
            .detach();
        }
        // Pending MCP/skills methods have no protocol number yet. They are not
        // offered or called until their owner supplies a reviewed capability.
        Integration::Tools | Integration::Skills => {
            settled(shell, operation, Outcome::Refused, window, cx);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn execute(
    shell: &mut Shell,
    operation: Operation,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    settled(shell, operation, Outcome::Refused, window, cx);
}

fn settled(
    shell: &mut Shell,
    operation: Operation,
    outcome: Outcome,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let setup = &mut shell.areas.onboarding.integrations;
    setup.running.retain(|running| *running != operation);
    match setup.model.settle(operation, outcome) {
        Settlement::Complete(visit) => shell.advance_setup(visit, window, cx),
        Settlement::Stale | Settlement::Waiting => {}
    }
    cx.notify();
}

#[cfg(not(target_arch = "wasm32"))]
fn install_query(setup: &mut Setup, ticket: Ticket, facts: Facts) -> bool {
    if setup.query != Some(ticket) {
        return false;
    }
    setup.facts = facts;
    true
}

#[cfg(not(target_arch = "wasm32"))]
fn install_mutation(setup: &mut Setup, facts: Facts) {
    // An authorized mutation outranks every earlier status read, including a
    // read from a flow that was closed/reopened while the operation continued.
    setup.query = None;
    setup.facts = facts;
}

fn failed(shell: &Shell) -> bool {
    match &shell.areas.onboarding.integrations.facts {
        #[cfg(not(target_arch = "wasm32"))]
        Facts::Failed(_) => true,
        Facts::Loading | Facts::Ready(_) => false,
        #[cfg(target_arch = "wasm32")]
        Facts::Unavailable => false,
    }
}

fn cli_choice(snapshot: Snapshot) -> Option<Choice> {
    if snapshot.availability != Availability::Bundled {
        return None;
    }
    let state = CliState::from(snapshot);
    if state.allows(CliOperation::Install) {
        Some(Choice::Off)
    } else if state.allows(CliOperation::Remove) {
        Some(Choice::On)
    } else {
        // A foreign destination authorizes neither install nor removal. It is
        // unsupported here, not an unknown value that Continue may overwrite.
        None
    }
}

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.desktop();
    let cli = matches!(
        shell.areas.onboarding.integrations.facts,
        Facts::Ready(snapshot) if cli_choice(snapshot).is_some()
    );
    let switch = cli_switch(shell, tokens, cx);
    div()
        .flex()
        .flex_col()
        .child(
            settings::card(tokens)
                .child(tools_row(shell, tokens, compact))
                .when(failed(shell), |card| {
                    card.child(retry_row(shell, tokens, cx))
                })
                .when(cli, |card| {
                    card.child(settings::row_at(
                        words.cli(),
                        Some(words.tools_note().into()),
                        None,
                        Some(switch),
                        false,
                        compact,
                        tokens,
                    ))
                })
                .when(
                    !shell
                        .areas
                        .onboarding
                        .integrations
                        .model
                        .failures()
                        .is_empty(),
                    |card| {
                        card.child(settings::row(
                            words.setup_failed(),
                            None,
                            None,
                            false,
                            tokens,
                        ))
                    },
                ),
        )
        .into_any_element()
}

fn tools_row(shell: &Shell, tokens: &Tokens, compact: bool) -> AnyElement {
    let words = shell.words.desktop();
    settings::row_at(
        words.orchestration(),
        Some(words.tools_unavailable().into()),
        None,
        Some(
            settings::switch("setup-tools-unavailable", true, false, false, 0, tokens)
                .into_any_element(),
        ),
        true,
        compact,
        tokens,
    )
    .into_any_element()
}

fn cli_switch(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    let on = shell
        .areas
        .onboarding
        .integrations
        .model
        .choice(Integration::Cli)
        == Choice::On;
    let enabled = !busy(shell);
    settings::switch("setup-cli-choice", on, enabled, busy(shell), 0, tokens)
        .on_click(cx.listener(move |shell, _event, _window, cx| {
            if !busy(shell) {
                let result = shell
                    .areas
                    .onboarding
                    .integrations
                    .model
                    .choose(Integration::Cli, if on { Choice::Off } else { Choice::On });
                if result.is_err() {
                    shell.notice = Some(shell.words.desktop().setup_failed().to_owned());
                }
            }
            cx.notify();
        }))
        .into_any_element()
}

fn retry_row(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    let words = shell.words.desktop();
    settings::row(
        words.setup_failed(),
        None,
        Some(
            outline_button(
                "setup-integration-retry",
                words.retry(),
                !busy(shell),
                tokens,
            )
            .on_click(cx.listener(|shell, _event, _window, cx| refresh(shell, cx)))
            .into_any_element(),
        ),
        false,
        tokens,
    )
    .into_any_element()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use vessel_app_view::desktop::cli::PathStatus;

    #[test]
    fn unsupported_cli_entries_never_become_deferred_operations() {
        for (availability, status, expected) in [
            (Availability::Missing, CliStatus::Absent, None),
            (Availability::Missing, CliStatus::Installed, None),
            (Availability::Bundled, CliStatus::Foreign, None),
            (Availability::Bundled, CliStatus::Absent, Some(Choice::Off)),
            (
                Availability::Bundled,
                CliStatus::Installed,
                Some(Choice::On),
            ),
        ] {
            assert_eq!(
                cli_choice(Snapshot {
                    availability,
                    status,
                    path: PathStatus::Manual
                }),
                expected
            );
        }
    }

    #[test]
    fn a_late_status_query_cannot_replace_an_authorized_mutation_result() {
        let mut setup = Setup::default();
        let ticket = setup.tickets.next_ticket().unwrap();
        setup.query = Some(ticket);
        let installed = Snapshot {
            availability: Availability::Bundled,
            status: CliStatus::Installed,
            path: PathStatus::Configured,
        };
        // The original install may finish after its old flow was abandoned.
        install_mutation(&mut setup, Facts::Ready(installed));
        let stale = Snapshot {
            status: CliStatus::Absent,
            ..installed
        };
        assert!(!install_query(&mut setup, ticket, Facts::Ready(stale)));
        assert!(matches!(setup.facts, Facts::Ready(current) if current == installed));
    }
}
