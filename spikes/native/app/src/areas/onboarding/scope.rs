//! Acknowledged authenticated target binding fences reused settings controls.

use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{
    Command,
    onboarding::{Answer, Ask},
};
use vessel_app_view::Reach;
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::devices::{Ticket, Tickets};
use vessel_app_view::onboarding::{
    Layout, State as ModelState, Visit,
    scope::{Problem, Target},
};

#[derive(Default)]
enum Resolution {
    #[default]
    Idle,
    Pending {
        ticket: Ticket,
        layout: Layout,
    },
    Ready,
    Failed,
}

#[derive(Default)]
enum Binding {
    #[default]
    Unbound,
    Pending(Visit),
    Ready(Visit),
    Failed {
        visit: Visit,
        problem: Problem,
    },
}

#[derive(Default)]
pub(super) struct ScopeState {
    tickets: Tickets,
    resolution: Resolution,
    binding: Binding,
    reopen: Option<(Target, Layout)>,
}

pub(super) fn prepare(shell: &mut Shell) {
    if let Some((target, layout)) = shell.areas.onboarding.scope.reopen.take()
        && shell.areas.onboarding.model.reopen(target, layout).is_err()
    {
        shell.notice = Some(shell.words.desktop().setup_failed().to_owned());
    }
    // Automatic resolution waits for C1's authoritative guarded inventory.
}

pub(super) fn manual(shell: &mut Shell, window: &Window) {
    let machine = super::selected_host(shell)
        .filter(|machine| machine.reach == Reach::Online)
        .and_then(vessel_app_view::Machine::connection_context);
    if let Some(machine) = machine {
        resolve(shell, machine, super::layout(window));
    } else {
        shell.notice = Some(shell.words.desktop().connect().to_owned());
    }
}

fn resolve(shell: &mut Shell, context: ConnectionContext, layout: Layout) {
    let Ok(ticket) = shell.areas.onboarding.scope.tickets.next_ticket() else {
        shell.notice = Some(shell.words.desktop().setup_failed().to_owned());
        return;
    };
    shell.areas.onboarding.scope.resolution = Resolution::Pending { ticket, layout };
    shell.command(Command::Onboarding(Ask::Resolve { context, ticket }));
}

pub(super) fn bind(shell: &mut Shell, target: Target, visit: Visit) {
    shell.areas.onboarding.scope.binding = Binding::Pending(visit);
    shell.command(Command::Onboarding(Ask::Bind { target, visit }));
}

pub(super) fn release(shell: &mut Shell, visit: Visit) {
    shell.areas.onboarding.scope.binding = Binding::Unbound;
    shell.command(Command::Onboarding(Ask::Release(visit)));
}

pub(super) fn actionable(shell: &Shell, cx: &gpui::App) -> bool {
    if !super::effects_allowed(cx) {
        return false;
    }
    let ModelState::Open(open) = shell.areas.onboarding.model.state() else {
        return false;
    };
    if open.step() == vessel_app_view::onboarding::Step::Projects {
        // Manual form readiness and checked-add settlement are separate C1
        // capabilities. Neither an empty list nor idle UI acknowledges them.
        return false;
    }
    matches!(shell.areas.onboarding.scope.binding, Binding::Ready(visit) if visit == open.visit())
        && shell.fleet.machines.iter().any(|machine| {
            machine.name == open.host().machine().as_str()
                && machine.reach == Reach::Online
                && machine.connection_context().as_ref() == Some(open.host().context())
        })
}

pub(super) fn hint(shell: &Shell) -> &str {
    if matches!(shell.areas.onboarding.model.state(), ModelState::Open(open)
        if open.step() == vessel_app_view::onboarding::Step::Projects)
    {
        return shell.words.desktop().projects_pending();
    }
    match shell.areas.onboarding.scope.binding {
        Binding::Failed { visit, problem } => {
            if matches!(shell.areas.onboarding.model.state(), ModelState::Open(open) if open.visit() == visit)
            {
                problem_text(shell, problem)
            } else {
                shell.words.desktop().binding()
            }
        }
        Binding::Pending(visit) => {
            if matches!(shell.areas.onboarding.model.state(), ModelState::Open(open) if open.visit() == visit)
            {
                shell.words.desktop().binding()
            } else {
                shell.words.desktop().setup_failed()
            }
        }
        Binding::Unbound | Binding::Ready(_) => shell.words.desktop().connect(),
    }
}

fn problem_text(shell: &Shell, problem: Problem) -> &str {
    match problem {
        Problem::Offline => shell.words.desktop().connect(),
        Problem::Changed => shell.words.desktop().host_changed(),
        Problem::Unavailable => shell.words.desktop().binding_unavailable(),
        Problem::Unsent => shell.words.desktop().setup_failed(),
    }
}

impl Shell {
    pub(crate) fn onboarding_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        match answer {
            Answer::Resolved { ticket, result } => {
                let Resolution::Pending {
                    ticket: current,
                    layout,
                } = self.areas.onboarding.scope.resolution
                else {
                    return;
                };
                if current != ticket {
                    return;
                }
                match result {
                    Ok(target) => {
                        if !self.fleet.machines.iter().any(|machine| {
                            machine.connection_context().as_ref() == Some(target.context())
                        }) {
                            self.areas.onboarding.scope.resolution = Resolution::Failed;
                            return;
                        }
                        self.areas.onboarding.scope.reopen = Some((target, layout));
                        self.areas.onboarding.scope.resolution = Resolution::Ready;
                    }
                    Err(problem) => {
                        self.areas.onboarding.scope.resolution = Resolution::Failed;
                        self.notice = Some(problem_text(self, problem).to_owned());
                    }
                }
            }
            Answer::Bound {
                target,
                visit,
                result,
            } => {
                let ModelState::Open(open) = self.areas.onboarding.model.state() else {
                    return;
                };
                if open.visit() != visit || open.host() != &target {
                    return;
                }
                let result = result.and_then(|()| {
                    self.fleet
                        .machines
                        .iter()
                        .any(|machine| {
                            machine.connection_context().as_ref() == Some(target.context())
                        })
                        .then_some(())
                        .ok_or(Problem::Changed)
                });
                self.areas.onboarding.scope.binding = match result {
                    Ok(()) => Binding::Ready(visit),
                    Err(problem) => Binding::Failed { visit, problem },
                };
            }
            Answer::Released => {}
        }
        cx.notify();
    }
}
