//! Original connection plus root travel through admission and queued replies.
//!
//! [Decision Log]
//! - Purpose: a book alias reused during I/O must never redirect a file intent.
//! - Constraint: session metadata is the existing root source; no cwd fallback.
//! - Choice: G4 admits once and fences every typed call; replies keep its context.
//! - Cost: retirement refuses old work, including reads; it never retries writes.
#[cfg(not(target_arch = "wasm32"))]
use super::file_call::Client;
use super::{FileAnswer, FileAsk};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Semaphore;
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::workspace::{
    file_scope::FileScope, file_ticket::FileTicket, problem::Problem,
};
use vessel_proto::{SessionId, SessionInfo, workspace::WorkspaceRoot};

#[cfg(not(target_arch = "wasm32"))]
const MAX_FILE_OPERATIONS: usize = 16;
#[cfg(not(target_arch = "wasm32"))]
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
    ///
    /// # Errors
    /// Returns `ScopeMismatch` when the request root differs from its scope.
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
    /// The browser demo has no file transport; settle without dispatch.
    #[cfg(target_arch = "wasm32")]
    #[must_use]
    pub(crate) fn unsupported(self) -> Reply {
        self.failed(&Problem::Older)
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
            guard: None,
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
    guard: Option<crate::DeliveryGuard>,
    outcome: Outcome,
}
impl Reply {
    /// The SAME original actor admission is checked immediately before data
    /// installation. A refusal still settles its original owner ticket.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.guard
            .as_ref()
            .is_some_and(|guard| guard.validate(&self.context).is_ok())
    }
    /// Settle the captured owner ticket, then fence installation against context.
    #[must_use]
    pub fn into_parts(self) -> (ConnectionContext, Outcome) {
        (self.context, self.outcome)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub(super) struct Hub {
    slots: Arc<Semaphore>,
    observations: Arc<Semaphore>,
}
#[cfg(not(target_arch = "wasm32"))]
impl Default for Hub {
    fn default() -> Self {
        Self {
            slots: Arc::new(Semaphore::new(MAX_FILE_OPERATIONS)),
            observations: Arc::new(Semaphore::new(MAX_FILE_OBSERVATIONS)),
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(crate) async fn scoped_file(&self, request: Request) {
        let selected = match self.selected_connection(&request.context) {
            Ok(selected) => selected,
            Err(why) => {
                self.emit(crate::Event::FileOperation(
                    request.failed(&Problem::Gone(why.to_string())),
                ))
                .await;
                return;
            }
        };
        let slots = match &request.work {
            Work::File { ask, .. } if matches!(ask.as_ref(), FileAsk::Observe { .. }) => {
                &self.workspace.file_calls.observations
            }
            Work::Root { .. } | Work::File { .. } => &self.workspace.file_calls.slots,
        };
        let Ok(permit) = Arc::clone(slots).try_acquire_owned() else {
            // Actor backpressure settles a bounded refusal, without spawning
            // unlimited full-queue workers that hold submitted write payloads.
            self.emit(crate::Event::FileOperation(request.refused()))
                .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let guard = selected.delivery_guard();
            let client = Client(&selected);
            let outcome = match request.work {
                Work::Root { session, ticket } => Outcome::Root {
                    session,
                    ticket,
                    result: client
                        .call(vessel_proto::GetSession { session })
                        .await
                        .and_then(|info| {
                            if info.id == session {
                                Ok(Box::new(info))
                            } else {
                                Err(Problem::Unsent)
                            }
                        }),
                },
                Work::File { scope, ask } => Outcome::File(Box::new(FileReply {
                    dispatched: true,
                    scope,
                    answer: super::file_run::run(&client, *ask).await,
                })),
            };
            let reply = Reply {
                context: request.context,
                guard: Some(guard),
                outcome,
            };
            // Preserve original terminal write evidence even after retirement.
            // The receiver rejects stale data using this guard but MUST settle
            // the matching original attempt; no send cancellation or replay.
            let _closed = events.send(crate::Event::FileOperation(reply)).await;
            drop(permit);
        }));
    }
}
fn root_of(ask: &FileAsk) -> WorkspaceRoot {
    match ask {
        FileAsk::Observe { ask, .. } => ask.root,
        FileAsk::Inspect { ask, .. } => ask.root,
        FileAsk::Intelligence(ask) => ask.root,
        FileAsk::List { ask, .. } => ask.root,
        FileAsk::Read { ask, .. } | FileAsk::EditorRead { ask, .. } => ask.root,
        FileAsk::Save(ask) => ask.root,
        FileAsk::Find(ask) | FileAsk::TrackedFind { ask, .. } => ask.root,
        FileAsk::Grep(ask) => ask.root,
        FileAsk::Path { change, .. } => change.root(),
    }
}
