//! Bind setup to live authenticated clients before reusing their controls.

use vessel_app_view::devices::{MachineKey, Ticket};
use vessel_app_view::onboarding::{
    Visit,
    scope::{Problem, Target},
};

/// Serial command ordering makes binding precede every setup action.
#[derive(Debug, Clone)]
pub enum Ask {
    /// Resolve an alias using the current authenticated connection.
    Resolve {
        /// Already parsed book key.
        machine: MachineKey,
        /// Identity retained through the UI queue.
        ticket: Ticket,
    },
    /// Controls remain disabled until this exact binding is acknowledged.
    Bind {
        /// Intended authenticated target.
        target: Target,
        /// Current step stay.
        visit: Visit,
    },
    /// A stale close cannot clear a newer guard.
    Release(Visit),
}

/// Acknowledgements retain identity through the asynchronous event queue.
#[derive(Debug, Clone)]
pub enum Answer {
    /// Live identity lookup completed.
    Resolved {
        /// The request answered.
        ticket: Ticket,
        /// Authenticated target or safe refusal.
        result: Result<Target, Problem>,
    },
    /// Controls may become active only for the same target and visit.
    Bound {
        /// Intended node; no alias fallback.
        target: Target,
        /// Acknowledged stay.
        visit: Visit,
        /// Binding outcome.
        result: Result<(), Problem>,
    },
    /// No host operation is performed by releasing a guard.
    Released,
}

impl Ask {
    /// Queue/browser refusal settles in the same response shape.
    #[must_use]
    pub fn refused(self, problem: Problem) -> Answer {
        match self {
            Self::Resolve { ticket, .. } => Answer::Resolved {
                ticket,
                result: Err(problem),
            },
            Self::Bind { target, visit } => Answer::Bound {
                target,
                visit,
                result: Err(problem),
            },
            Self::Release(_) => Answer::Released,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(crate) async fn onboarding_agents(
        &self,
        machine: crate::agents::MachineKey,
        request: vessel_proto::RequestId,
        ask: crate::agents::Ask,
    ) {
        let client = self
            .clients
            .get(machine.as_str())
            .map(|connected| connected.client.clone());
        if let Some(client) = &client {
            if let Err(problem) = self
                .onboarding_scope
                .check(&MachineKey::from_book(machine.as_str()), client.node().id)
            {
                if let Some((target, visit)) = self.onboarding_scope.binding() {
                    self.emit(crate::Event::Onboarding(Answer::Bound {
                        target: target.clone(),
                        visit,
                        result: Err(problem),
                    }))
                    .await;
                }
                self.emit(crate::Event::Agents {
                    machine,
                    request,
                    answer: Err(crate::agents::AgentsError::Setup(problem)),
                })
                .await;
                return;
            }
        }
        self.agents
            .ask(client, machine, request, ask, self.events.clone())
            .await;
    }

    pub(crate) async fn onboarding(&mut self, ask: Ask) {
        let answer = match ask {
            Ask::Resolve { machine, ticket } => {
                let result = self
                    .clients
                    .get(machine.as_str())
                    .map(|connected| Target::authenticated(machine, connected.client.node().id))
                    .ok_or(Problem::Offline);
                Answer::Resolved { ticket, result }
            }
            Ask::Bind { target, visit } => {
                let result = self
                    .clients
                    .get(target.machine().as_str())
                    .ok_or(Problem::Offline)
                    .and_then(|connected| {
                        if connected.client.node().id == target.node() {
                            Ok(())
                        } else {
                            Err(Problem::Changed)
                        }
                    });
                if result.is_ok() {
                    self.onboarding_scope.bind(target.clone(), visit);
                }
                Answer::Bound {
                    target,
                    visit,
                    result,
                }
            }
            Ask::Release(visit) => {
                self.onboarding_scope.release(visit);
                Answer::Released
            }
        };
        self.emit(crate::Event::Onboarding(answer)).await;
    }
}
