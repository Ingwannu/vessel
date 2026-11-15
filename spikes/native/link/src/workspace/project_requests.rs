//! Project operations retain the shared physical admission through their result.
//!
//! [Decision Log]
//! - Purpose: reconnecting the same node must not inherit an old form's intent.
//! - Constraint: a write may already have taken effect when its connection retires.
//! - Alternatives: compare only NodeId, or allocate another area-local epoch.
//! - Choice: reuse SelectedConnection, echo its context and retain uncertainty.
//! - Impact: callers reconcile retired writes; no automatic replay is permitted.

#[cfg(not(target_arch = "wasm32"))]
use super::calls;
use super::{Answer, ProjectChange, ProjectFailure};
#[cfg(not(target_arch = "wasm32"))]
use crate::connections::{RequestFailure, SelectedConnection};
#[cfg(not(target_arch = "wasm32"))]
use vessel_app_view::workspace::projects::actions::{ProjectAction, ProjectChanged};
use vessel_app_view::{connection::ConnectionContext, workspace::problem::Problem};
#[cfg(not(target_arch = "wasm32"))]
use vessel_proto::workspace::{ListProjects, RemoveProject};

/// Only C1-owned project reads and mutations use this boundary.
#[derive(Debug)]
pub enum Operation {
    /// Authoritative list, correlated independently of a form.
    List(vessel_app_view::workspace::projects::reads::ProjectReadKey),
    /// A person-submitted registration, clone, init or removal.
    Change(ProjectChange),
}

/// The original machine snapshot, never resolved by NodeId or another alias.
#[derive(Debug)]
pub struct Request {
    /// Shared authenticated admission captured before dispatch.
    pub context: ConnectionContext,
    /// Typed operation and its correlation ticket.
    pub operation: Operation,
}

/// A response cannot hydrate a replacement admission under the same alias.
#[derive(Debug)]
pub struct Response {
    /// Exact original admission, including reconnect generation.
    pub context: ConnectionContext,
    /// Existing typed workspace result.
    pub answer: Answer,
}

impl Request {
    /// A rejected command has certainly not reached the actor or node.
    #[must_use]
    pub fn refused(self) -> Response {
        Response {
            context: self.context,
            answer: refused(self.operation, ProjectFailure::Call(Problem::Unsent)),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Operation {
    fn matches(&self, context: &ConnectionContext) -> bool {
        let (host, node) = match self {
            Self::List(key) => (key.target().host(), key.target().node()),
            Self::Change(change) => (change.key.host(), change.key.node()),
        };
        host.as_str() == context.machine().as_str() && node == context.node()
    }
}

const MAX_PROJECT_CALLS: usize = 16;
#[cfg(not(target_arch = "wasm32"))]
const PROJECT_CALL_DEADLINE: std::time::Duration = std::time::Duration::from_secs(60);

type FollowKey = (ConnectionContext, vessel_proto::ProjectId);
type Following = std::sync::Arc<std::sync::Mutex<Vec<FollowKey>>>;

#[derive(Debug, Clone)]
pub(crate) struct Calls {
    permits: std::sync::Arc<tokio::sync::Semaphore>,
    following: Following,
}
impl Default for Calls {
    fn default() -> Self {
        Self {
            permits: std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_PROJECT_CALLS)),
            following: std::sync::Arc::default(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    /// Older workspace resources retain their existing dispatch. C1 project
    /// operations must use their captured admission rather than raw alias IO.
    pub(crate) fn workspace(&self, machine: String, ask: super::Ask) {
        let events = self.events.clone();
        if matches!(
            &ask,
            super::Ask::ProjectChange(_)
                | super::Ask::Project(
                    super::ProjectAsk::List(_)
                        | super::ProjectAsk::Add(_)
                        | super::ProjectAsk::Clone(_)
                        | super::ProjectAsk::Init(_)
                        | super::ProjectAsk::Remove(_)
                )
        ) {
            super::Hub::refuse(machine, ask, Problem::Unsent, events);
            return;
        }
        match self.client(&machine) {
            Ok(client) => self.workspace.ask(client.clone(), machine, ask, events),
            Err(error) => {
                let problem = Problem::Gone(error.to_string());
                super::Hub::refuse(machine, ask, problem, events);
            }
        }
    }

    pub(crate) async fn projects(&self, request: Request) {
        let Request { context, operation } = request;
        if !operation.matches(&context) {
            self.emit(crate::Event::Projects(Response {
                context,
                answer: refused(operation, ProjectFailure::HostChanged),
            }))
            .await;
            return;
        }
        let selected = self.selected_connection(&context);
        let permit =
            std::sync::Arc::clone(&self.workspace.project_calls.permits).try_acquire_owned();
        let (selected, permit) = match (selected, permit) {
            (Ok(selected), Ok(permit)) => (selected, permit),
            (selected, permit) => {
                let failure = if selected.is_err() {
                    ProjectFailure::HostChanged
                } else {
                    ProjectFailure::Call(Problem::Unsent)
                };
                drop(permit);
                self.emit(crate::Event::Projects(Response {
                    context,
                    answer: refused(operation, failure),
                }))
                .await;
                return;
            }
        };
        let events = self.events.clone();
        let calls = self.workspace.project_calls.clone();
        drop(tokio::spawn(async move {
            let answer = run(&selected, operation).await;
            observe::start(&calls, &selected, &context, &answer, &events);
            let _closed = events
                .send(crate::Event::Projects(Response { context, answer }))
                .await;
            drop(permit);
        }));
    }
}

fn refused(operation: Operation, failure: ProjectFailure) -> Answer {
    match operation {
        Operation::List(key) => Answer::Projects {
            key,
            result: Err(Problem::Unsent),
        },
        Operation::Change(change) => Answer::ProjectChanged {
            key: change.key,
            result: Err(failure),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn run(client: &SelectedConnection, operation: Operation) -> Answer {
    match operation {
        Operation::List(key) => Answer::Projects {
            key,
            result: match tokio::time::timeout(PROJECT_CALL_DEADLINE, client.call(ListProjects {}))
                .await
            {
                Ok(result) => result.map_err(read_failure),
                Err(_deadline) => Err(Problem::Unsent),
            },
        },
        Operation::Change(change) => {
            let result =
                tokio::time::timeout(PROJECT_CALL_DEADLINE, send(client, change.action)).await;
            let result = match result {
                Ok(result) => result,
                Err(_deadline) => Err(ProjectFailure::TimedOut),
            };
            Answer::ProjectChanged {
                key: change.key,
                result,
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn send(
    client: &SelectedConnection,
    action: ProjectAction,
) -> Result<ProjectChanged, ProjectFailure> {
    match action {
        ProjectAction::Register(ask) => client.call(ask).await.map(ProjectChanged::Registered),
        ProjectAction::Clone(ask) => client.call(ask).await.map(ProjectChanged::Registered),
        ProjectAction::Initialize(ask) => client.call(ask).await.map(ProjectChanged::Registered),
        ProjectAction::Remove(project) => client
            .call(RemoveProject { project })
            .await
            .map(|_ack| ProjectChanged::Removed(project)),
    }
    .map_err(|failure| match failure {
        // SelectedConnection may retire after first poll. Unlike actor preflight,
        // this cannot establish that a remote write was never sent.
        RequestFailure::Refused(_) => ProjectFailure::OutcomeUnknown,
        RequestFailure::Call(error) => ProjectFailure::Call(calls::problem(&error)),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn read_failure(failure: RequestFailure) -> Problem {
    match failure {
        RequestFailure::Refused(_) => Problem::Unsent,
        RequestFailure::Call(error) => calls::problem(&error),
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod observe;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
