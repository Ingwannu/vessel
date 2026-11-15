//! One bounded cache over the existing lazy summary/file readers. Active work
//! belongs to the node tracker, independently of cached payloads and callers.
//!
//! [Decision Log]
//! - Purpose: identical viewers share work and a forced refresh cannot publish
//!   an older result after a repository edit.
//! - Existing assets: Jobs retains cleanup, Tokio watch delivers shared results,
//!   and the protected comparison reader owns every git command/private index.
//! - Choice: a small synchronous state machine plus owned tracked work; no lock
//!   is held across I/O. Cancellation stops commands, never drops their cleanup.
//! - Difference: Vessel's lazy typed file pages have separate identities within
//!   the same 64-payload budget instead of caching a whole text patch. This keeps
//!   the existing parser/wire contract and bounds unopened-file work.

mod core;
mod step;

use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::ProjectId;
use vessel_proto::workspace::{DiffSummary, FileDiff, GitCompared};

use super::jobs::Jobs;
use crate::workspace::WorkspaceError;

pub(in crate::workspace) use self::core::Change;
pub(in crate::workspace::comparison) use self::core::{Key, ReadReason, Subject};

/// A shared result retains typed errors and their original source chains.
pub(in crate::workspace::comparison) type Reply = Result<Arc<Snapshot>, Arc<WorkspaceError>>;
type Receiver = watch::Receiver<Option<Reply>>;

#[derive(Debug, Clone)]
pub(in crate::workspace::comparison) enum Snapshot {
    Summary(GitCompared<DiffSummary>),
    File(GitCompared<FileDiff>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PayloadKind {
    Summary,
    File,
}

impl Snapshot {
    pub(in crate::workspace::comparison) const fn kind(&self) -> PayloadKind {
        match self {
            Self::Summary(_) => PayloadKind::Summary,
            Self::File(_) => PayloadKind::File,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CacheError {
    #[error(transparent)]
    Refused(#[from] core::Refusal),
    #[error("the comparison result channel closed")]
    Channel(#[from] watch::error::RecvError),
    #[error("the comparison result has type {received:?}, expected {expected:?}")]
    Payload {
        expected: PayloadKind,
        received: PayloadKind,
    },
    #[error("the comparison worker has no retained owner")]
    Ownership,
}

/// Root authorization is performed again by the caller before every probe.
#[derive(Debug, Default)]
pub(in crate::workspace) struct Reader {
    jobs: Jobs,
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    cache: core::Cache<Snapshot>,
    active: Vec<Active>,
}

#[derive(Debug)]
struct Active {
    id: core::BuildId,
    cancel: CancellationToken,
    reply: Receiver,
}

/// Existing node lifecycle, not a per-connection task or cancellation owner.
#[derive(Clone, Copy)]
pub(in crate::workspace::comparison) struct Owner<'a> {
    pub(in crate::workspace::comparison) tasks: &'a TaskTracker,
    pub(in crate::workspace::comparison) stop: &'a CancellationToken,
}

#[derive(Debug)]
pub(in crate::workspace::comparison) enum ReadStep {
    Complete(Reply),
    Retry,
}

/// A pending invalidated build is awaited through cleanup before revalidation.
enum Delivery {
    Complete(Reply),
    Wait { reply: Receiver, mode: WaitMode },
}
enum WaitMode {
    Join,
    After,
}

impl Reader {
    /// History shares the existing admission/execution owner without pretending
    /// its result has comparison cache identity or connection ownership.
    pub(in crate::workspace) async fn owned<T, F>(
        &self,
        tasks: &TaskTracker,
        stop: CancellationToken,
        work: F,
    ) -> Result<T, super::jobs::JobError>
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        self.jobs.submit(tasks, stop, work)?.result().await
    }

    pub(in crate::workspace::comparison) async fn read<F, Load>(
        &self,
        key: Key,
        reason: ReadReason,
        owner: Owner<'_>,
        load: Load,
    ) -> Result<ReadStep, CacheError>
    where
        F: Future<Output = Result<Snapshot, WorkspaceError>> + Send + 'static,
        Load: FnOnce(CancellationToken) -> F,
    {
        let delivery = self.prepare(key, reason, owner, load)?;
        match delivery {
            Delivery::Complete(reply) => Ok(ReadStep::Complete(reply)),
            Delivery::Wait { mut reply, mode } => {
                let value = loop {
                    if let Some(value) = reply.borrow_and_update().clone() {
                        break value;
                    }
                    reply.changed().await?;
                };
                match mode {
                    WaitMode::Join => Ok(ReadStep::Complete(value)),
                    WaitMode::After => Ok(ReadStep::Retry),
                }
            }
        }
    }

    fn invalidate(
        &self,
        root: vessel_proto::workspace::WorkspaceRoot,
        project: ProjectId,
        change: Change,
    ) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let cancelled = state.cache.invalidate(root, project, change);
        for active in &state.active {
            if cancelled.contains(&active.id) {
                active.cancel.cancel();
            }
        }
    }
}

#[cfg(test)]
mod tests;

/// Invalidates both sides of a write, even when it partially changes git and
/// then returns a refusal/error or its caller drops the future.
#[must_use]
pub(in crate::workspace) struct Mutation<'a> {
    reader: &'a Reader,
    root: vessel_proto::workspace::WorkspaceRoot,
    project: ProjectId,
    change: Change,
}

impl Reader {
    pub(in crate::workspace) fn mutating(
        &self,
        root: vessel_proto::workspace::WorkspaceRoot,
        project: ProjectId,
        change: Change,
    ) -> Mutation<'_> {
        self.invalidate(root, project, change);
        Mutation {
            reader: self,
            root,
            project,
            change,
        }
    }
}
impl Drop for Mutation<'_> {
    fn drop(&mut self) {
        self.reader.invalidate(self.root, self.project, self.change);
    }
}

#[cfg(test)]
impl Reader {
    // Observe actual storage, not a TTL-sensitive second read or a fake cache.
    pub(in crate::workspace) fn cached_payloads_for_test(&self) -> usize {
        self.state.lock().unwrap().cache.completed_for_test()
    }
    pub(in crate::workspace) fn hold_execution_for_test(
        &self,
    ) -> tokio::sync::OwnedSemaphorePermit {
        self.jobs.hold_execution_for_test()
    }
    pub(in crate::workspace) fn submitted_for_test(&self) -> &tokio::sync::Notify {
        self.jobs.submitted_for_test()
    }
}
