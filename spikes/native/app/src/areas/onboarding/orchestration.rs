//! Deferred integration choices keep their visit and batch through actual owner acknowledgements.

use super::{Context, Shell, Visit, Window};
use std::collections::BTreeMap;
use vessel_app_kit::ui::{
    AnyElement, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div,
};
use vessel_app_kit::{Tokens, outline_button, settings};
use vessel_app_view::desktop::cli::{Availability, Snapshot, Status as CliStatus};
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
                if shell.areas.onboarding.integrations.query != Some(ticket) {
                    return;
                }
                shell.areas.onboarding.integrations.facts = match result {
                    Ok(snapshot) => Facts::Ready(snapshot),
                    Err(error) => Facts::Failed(error),
                };
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
    if busy(shell) {
        return;
    }
    let mut offered = BTreeMap::new();
    if let Facts::Ready(snapshot) = shell.areas.onboarding.integrations.facts
        && snapshot.availability == Availability::Bundled
    {
        let current = match snapshot.status {
            CliStatus::Installed => Some(Choice::On),
            CliStatus::Absent => Some(Choice::Off),
            CliStatus::Foreign => None,
        };
        let _old = offered.insert(Integration::Cli, current);
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
    use vessel_app_view::desktop::cli::Operation as CliOperation;
    match operation.integration {
        Integration::Cli => {
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
                            shell.areas.onboarding.integrations.facts = Facts::Ready(snapshot);
                            Outcome::Applied
                        }
                        Err(error) => {
                            shell.areas.onboarding.integrations.facts = Facts::Failed(error);
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

fn failed(shell: &Shell) -> bool {
    match &shell.areas.onboarding.integrations.facts {
        #[cfg(not(target_arch = "wasm32"))]
        Facts::Failed(_) => true,
        Facts::Loading | Facts::Ready(_) => false,
        #[cfg(target_arch = "wasm32")]
        Facts::Unavailable => false,
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
        Facts::Ready(Snapshot {
            availability: Availability::Bundled,
            ..
        })
    );
    let on = shell
        .areas
        .onboarding
        .integrations
        .model
        .choice(Integration::Cli)
        == Choice::On;
    let enabled = !busy(shell);
    let switch = settings::switch("setup-cli-choice", on, enabled, busy(shell), 0, tokens)
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
        .into_any_element();
    div()
        .flex()
        .flex_col()
        .child(
            settings::card(tokens)
                .child(settings::row_at(
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
                ))
                .when(failed(shell), |card| {
                    card.child(settings::row(
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
                    ))
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
