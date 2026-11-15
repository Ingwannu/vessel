//! Original connection plus root travel through admission and queued replies.
//!
//! [Decision Log]
//! - Purpose: a book alias reused during I/O must never redirect a file intent.
//! - Constraint: session metadata is the existing root source; no cwd fallback.
//! - Choice: G4 admits once and fences every typed call; replies keep its context.
//! - Cost: retirement refuses old work, including reads; it never retries writes.
use super::{FileAnswer, FileAsk, file_call::Client};
use std::sync::Arc;
use tokio::sync::Semaphore;
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::workspace::{
    file_scope::FileScope, file_ticket::FileTicket, problem::Problem,
};
use vessel_proto::{SessionId, SessionInfo, workspace::WorkspaceRoot};

const MAX_FILE_OPERATIONS: usize = 16;
const MAX_FILE_OBSERVATIONS: usize = 64;

/// One request captured before submission. Constructors cannot change its root.
#[derive(Debug)]
pub struct Request {
    context: ConnectionContext,
    work: Work,
}
#[derive(Debug)]
enum Work {
    Root {
        session: SessionId,
        ticket: FileTicket,
    },
    File {
        scope: FileScope,
        ask: Box<FileAsk>,
    },
}
impl Request {
    /// Resolve metadata for this exact admitted session, never the selected tab.
    #[must_use]
    pub fn root(context: ConnectionContext, session: SessionId, ticket: FileTicket) -> Self {
        Self {
            context,
            work: Work::Root { session, ticket },
        }
    }
    /// Refuse a root mismatch before the actor or node can observe the request.
    pub fn file(scope: FileScope, ask: FileAsk) -> Result<Self, ScopeMismatch> {
        if root_of(&ask) != scope.root() {
            return Err(ScopeMismatch);
        }
        Ok(Self {
            context: scope.connection().clone(),
            work: Work::File {
                scope,
                ask: Box::new(ask),
            },
        })
    }
    /// Settle original tickets when the command queue refused the operation.
    #[must_use]
    pub fn refused(self) -> Reply {
        self.failed(&Problem::Unsent)
    }
    fn failed(self, problem: &Problem) -> Reply {
        let outcome = match self.work {
            Work::Root { session, ticket } => Outcome::Root {
                session,
                ticket,
                result: Err(problem.clone()),
            },
            Work::File { scope, ask } => Outcome::File(Box::new(FileReply {
                dispatched: false,
                scope,
                answer: (*ask).refused(problem),
            })),
        };
        Reply {
            context: self.context,
            outcome,
        }
    }
}
/// The request names another physical root than its captured scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("file request root differs from its original scope")]
pub struct ScopeMismatch;

/// Closed owner receipt. Private fields prevent unrelated workspace answers.
#[derive(Debug)]
pub struct FileReply {
    dispatched: bool,
    scope: FileScope,
    answer: FileAnswer,
}
impl FileReply {
    /// Existing file reducer can be reused after checking the original scope.
    #[must_use]
    pub fn into_parts(self) -> (FileScope, FileAnswer, bool) {
        (self.scope, self.answer, self.dispatched)
    }
}
/// Exact operation result; correlation is not a permission to install stale data.
#[derive(Debug)]
pub enum Outcome {
    /// Session metadata captured under the original live connection.
    Root {
        /// Original session.
        session: SessionId,
        /// Original metadata acquisition.
        ticket: FileTicket,
        /// Root-bearing metadata or refusal.
        result: Result<Box<SessionInfo>, Problem>,
    },
    /// A root-confined file operation from the one existing implementation.
    File(Box<FileReply>),
}
/// Replies preserve the original context even when admission or polling failed.
#[derive(Debug)]
pub struct Reply {
    context: ConnectionContext,
    outcome: Outcome,
}
impl Reply {
    /// Settle the captured owner ticket, then fence installation against context.
    #[must_use]
    pub fn into_parts(self) -> (ConnectionContext, Outcome) {
        (self.context, self.outcome)
    }
}

#[derive(Debug, Clone)]
pub(super) struct Hub {
    slots: Arc<Semaphore>,
    observations: Arc<Semaphore>,
}
impl Default for Hub {
    fn default() -> Self {
        Self {
            slots: Arc::new(Semaphore::new(MAX_FILE_OPERATIONS)),
            observations: Arc::new(Semaphore::new(MAX_FILE_OBSERVATIONS)),
        }
    }
}
impl crate::State {
    pub(crate) fn scoped_file(&self, request: Request) {
        let selected = self.selected_connection(&request.context);
        let slots = match &request.work {
            Work::File { ask, .. } if matches!(ask.as_ref(), FileAsk::Observe { .. }) => {
                &self.workspace.file_calls.observations
            }
            Work::Root { .. } | Work::File { .. } => &self.workspace.file_calls.slots,
        };
        let slot = slots.clone().try_acquire_owned();
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let reply = match (selected, slot) {
                (Ok(selected), Ok(_slot)) => {
                    let client = Client(&selected);
                    let outcome = match request.work {
                        Work::Root { session, ticket } => Outcome::Root {
                            session,
                            ticket,
                            result: client
                                .call(vessel_proto::GetSession { session })
                                .await
                                .map(Box::new),
                        },
                        Work::File { scope, ask } => Outcome::File(Box::new(FileReply {
                            dispatched: true,
                            scope,
                            answer: super::file_run::run(&client, *ask).await,
                        })),
                    };
                    Reply {
                        context: request.context,
                        outcome,
                    }
                }
                (Err(why), _) => request.failed(&Problem::Gone(why.to_string())),
                (Ok(_selected), Err(_full)) => request.refused(),
            };
            let _closed = events.send(crate::Event::FileOperation(reply)).await;
        }));
    }
}
fn root_of(ask: &FileAsk) -> WorkspaceRoot {
    match ask {
        FileAsk::Observe { ask, .. } => ask.root,
        FileAsk::Intelligence(ask) => ask.root,
        FileAsk::List { ask, .. } => ask.root,
        FileAsk::Read { ask, .. } | FileAsk::EditorRead { ask, .. } => ask.root,
        FileAsk::Save(ask) => ask.root,
        FileAsk::Find(ask) | FileAsk::TrackedFind { ask, .. } => ask.root,
        FileAsk::Grep(ask) => ask.root,
        FileAsk::Path { change, .. } => change.root(),
    }
}
