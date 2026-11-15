//! Bind setup to live authenticated clients before reusing their controls.

use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::devices::Ticket;
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
        context: ConnectionContext,
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
    pub(crate) fn onboarding_client(
        &self,
        machine: &str,
    ) -> Result<&vessel_client::NodeClient, crate::LinkError> {
        let connected = self
            .clients
            .get(machine)
            .ok_or_else(|| crate::LinkError::NotConnected(machine.to_owned()))?;
        self.onboarding_scope
            .check_control(&connected.context)
            .map_err(crate::LinkError::Onboarding)?;
        Ok(&connected.client)
    }

    pub(crate) fn onboarding_project_client(
        &self,
        machine: &str,
        ask: &crate::workspace::Ask,
    ) -> Result<&vessel_client::NodeClient, crate::LinkError> {
        use crate::workspace::{Ask, ProjectAsk};
        match ask {
            Ask::Project(
                ProjectAsk::Add(_)
                | ProjectAsk::Clone(_)
                | ProjectAsk::Init(_)
                | ProjectAsk::Remove(_)
                | ProjectAsk::Approve(_),
            ) => self.onboarding_client(machine),
            Ask::Layout(_)
            | Ask::Project(ProjectAsk::List | ProjectAsk::Commands(_))
            | Ask::Worktree(_)
            | Ask::Git(_)
            | Ask::File(_)
            | Ask::Start(_)
            | Ask::Session(_)
            | Ask::Pull(_) => self.client(machine),
        }
    }

    pub(crate) async fn onboarding_agents(
        &self,
        machine: crate::agents::MachineKey,
        request: vessel_proto::RequestId,
        ask: crate::agents::Ask,
    ) {
        let connected = self.clients.get(machine.as_str());
        if let Some(connected) = connected
            && let Err(problem) = self.onboarding_scope.check_control(&connected.context)
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
        self.agents
            .ask(
                connected.map(|connected| connected.client.clone()),
                machine,
                request,
                ask,
                self.events.clone(),
            )
            .await;
    }

    pub(crate) async fn onboarding(&mut self, ask: Ask) {
        let answer = match ask {
            Ask::Resolve { context, ticket } => {
                let result = self
                    .selected_connection(&context)
                    .map(|_selected| Target::authenticated(context))
                    .map_err(setup_refusal);
                Answer::Resolved { ticket, result }
            }
            Ask::Bind { target, visit } => {
                let result = self
                    .selected_connection(target.context())
                    .map(|_selected| ())
                    .map_err(setup_refusal);
                // Even a refused binding owns this visit's protective fence.
                // Otherwise closing a failed new visit would leave the old
                // visit's guard installed, because stale release is refused.
                self.onboarding_scope.bind(target.clone(), visit);
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

#[cfg(not(target_arch = "wasm32"))]
fn setup_refusal(refusal: vessel_app_view::connection::Refusal) -> Problem {
    match refusal {
        vessel_app_view::connection::Refusal::Changed => Problem::Changed,
        vessel_app_view::connection::Refusal::Closed => Problem::Offline,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use vessel_app_view::connection::Refusal;

    #[test]
    fn closed_transport_is_not_reported_as_an_alias_replacement() {
        assert_eq!(setup_refusal(Refusal::Closed), Problem::Offline);
        assert_eq!(setup_refusal(Refusal::Changed), Problem::Changed);
    }
}
