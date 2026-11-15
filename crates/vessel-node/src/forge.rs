//! Pull requests (protocol 11, ADR 0029): which sessions are watched, the
//! task that watches them, what is sent back to a session, and what the
//! calls do. The provider (`vessel-forge`) makes the requests; this module
//! decides when, for whom, and what becomes of the answers.
//!
//! [Decision Log]
//! - Purpose: a live session placed in a project or a worktree has a
//!   branch, and its branch may have a pull request whose facts fill the
//!   board's three right-hand lanes (ADR 0019); a person may have its
//!   failing checks and reviews sent to the session.
//! - Constraint: one node may run many sessions, each request to a forge
//!   counts against a quota, and a forge that is slow or down must never
//!   hold a session or a call (the brief's rule).
//! - Choice: one task ([`watcher`]) owns every watch, keyed by repository
//!   and branch, and is told when a session starts, is resumed, is
//!   archived or is deleted; it notices an end at the session's next turn
//!   in its schedule. Every read and write runs as a job of its own,
//!   bounded, whose answer comes back as a message, so the owner never
//!   waits on the network. A session's view reaches its actor through the
//!   session's own watch (`SessionHandle::set_pull_request`), where the
//!   one reducer places it.
//! - The calls that act (`pr.open`, `pr.merge`) run in the caller's call,
//!   not in the watcher ([`calls`]): they are a person's (or an agent's)
//!   and wait for the forge; they tell the watcher to look again once
//!   done.

mod answers;
mod calls;
mod feedback;
mod folder;
mod hosts;
mod poll;
// ADR 0061 stages real service tests before transport wiring is assigned.
#[cfg(test)]
mod project;
mod schedule;
mod view;
mod watcher;
mod watches;

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_forge::{EnvTokens, Forge};
use vessel_harness::Lookup;
use vessel_proto::forge::{ForgeStatus, PullRequestList, PullRequestView, SessionPullRequest};
use vessel_proto::{ErrorCode, ProjectId, SessionId, WireError};
use vessel_session::SessionHandle;
use vessel_store::Store;

use crate::workspace::Workspaces;

pub(crate) use watcher::MAX_WATCHES;

/// How a node reaches forges: from its configuration.
#[derive(Debug, Clone, Default)]
pub struct ForgeSetup {
    /// Tokens the binary found in the node's environment.
    pub env: EnvTokens,
    /// The machine's `gh`, found by the binary on its search path.
    pub gh: Option<PathBuf>,
    /// One forge for every host instead of the providers (the tests'
    /// in-memory forge). `dyn`: the trait's open extension point.
    pub custom: Option<Arc<dyn Forge>>,
}

impl ForgeSetup {
    /// What a binary finds: the tokens of the environment `read` gives
    /// (`GH_TOKEN`, `GITHUB_TOKEN`; `GH_ENTERPRISE_TOKEN` or
    /// `GITHUB_ENTERPRISE_TOKEN` for the host `GH_HOST` names) and `gh` on
    /// `lookup`'s path. The library reads no environment: the binary
    /// passes its own.
    #[must_use]
    pub fn found(read: impl Fn(&str) -> Option<OsString>, lookup: &Lookup) -> Self {
        Self {
            env: EnvTokens::from_vars(read),
            gh: lookup.find("gh"),
            custom: None,
        }
    }
}

/// What the watcher is told and asked.
#[derive(Debug)]
pub(crate) enum Command {
    /// A session started, or started again: watch it if it can be.
    Started(Box<Started>),
    /// A session was archived or deleted: drop it now.
    Forget(SessionId),
    /// Look at a session's pull request now (a refresh, or after an open
    /// or a merge), answering once looked; `None` when it is not held.
    Refresh {
        session: SessionId,
        /// A person waits (the look goes first).
        person: bool,
        reply: Option<oneshot::Sender<Option<PullRequestView>>>,
    },
    /// A session's pull request as held now.
    Get {
        session: SessionId,
        reply: oneshot::Sender<Option<PullRequestView>>,
    },
    /// Every watched session's.
    List {
        reply: oneshot::Sender<PullRequestList>,
    },
    /// Feedback turned on or off (already in the store).
    Feedback {
        session: SessionId,
        on: bool,
        reply: oneshot::Sender<Option<SessionPullRequest>>,
    },
}

/// A session the watcher may watch.
#[derive(Debug)]
pub(crate) struct Started {
    pub(crate) handle: SessionHandle,
    /// Its folder.
    pub(crate) folder: PathBuf,
    /// Its project.
    pub(crate) project: ProjectId,
    /// A person started it (its watch goes first).
    pub(crate) person: bool,
}

/// The node's forge part: the watcher's address and what calls need.
#[derive(Debug)]
pub(crate) struct Forges {
    commands: mpsc::Sender<Command>,
    hosts: Arc<hosts::Hosts>,
    workspaces: Arc<Workspaces>,
    store: Store,
    watched: Arc<AtomicU32>,
}

/// How many commands may wait for the watcher; past that a call answers
/// `unavailable` and a start is not watched (it is placed as today).
const COMMANDS: usize = 64;

impl Forges {
    /// Start the watcher as one of the node's tasks.
    pub(crate) fn start(
        setup: ForgeSetup,
        data: PathBuf,
        store: Store,
        workspaces: Arc<Workspaces>,
        tasks: &TaskTracker,
        stop: CancellationToken,
    ) -> Arc<Self> {
        let hosts = Arc::new(hosts::Hosts::new(setup, data));
        let (commands, received) = mpsc::channel(COMMANDS);
        let watched = Arc::new(AtomicU32::new(0));
        let task = watcher::Watcher::new(
            Arc::clone(&hosts),
            store.clone(),
            Arc::clone(&workspaces),
            received,
            Arc::clone(&watched),
        );
        drop(tasks.spawn(task.run(stop)));
        Arc::new(Self {
            commands,
            hosts,
            workspaces,
            store,
            watched,
        })
    }

    /// Tell the watcher, without waiting: a full channel drops the news
    /// (said in a log), which costs a session its pull request until it
    /// starts again, never the node a wait.
    pub(crate) fn tell(&self, command: Command) {
        if let Err(error) = self.commands.try_send(command) {
            tracing::warn!(%error, "the pull request watcher is busy; a change was not told");
        }
    }

    /// Ask the watcher and wait for its answer.
    ///
    /// # Errors
    /// `unavailable` when its channel is full or it stopped.
    pub(crate) async fn ask<T>(
        &self,
        command: impl FnOnce(oneshot::Sender<T>) -> Command,
    ) -> Result<T, WireError> {
        let (reply, answer) = oneshot::channel();
        self.commands
            .try_send(command(reply))
            .map_err(|_full| busy())?;
        answer.await.map_err(|_stopped| busy())
    }

    /// `forge.status`.
    pub(crate) async fn status(&self) -> ForgeStatus {
        let limit = u32::try_from(MAX_WATCHES).unwrap_or(u32::MAX);
        self.hosts
            .status(self.watched.load(Ordering::Relaxed), limit)
            .await
    }

    /// The project a session's place is in (for `Started`).
    pub(crate) fn project_of(
        &self,
        place: &vessel_proto::workspace::SessionPlace,
    ) -> Option<ProjectId> {
        self.workspaces.project_of(place)
    }
}

fn busy() -> WireError {
    WireError::new(
        ErrorCode::Unavailable,
        "the node's pull request watcher is busy or stopping; try again",
    )
}

impl Forges {
    /// Turn `session`'s feedback on or off in the store.
    pub(crate) async fn set_feedback(
        &self,
        session: SessionId,
        on: bool,
    ) -> Result<(), vessel_store::StoreError> {
        self.store.set_feedback(session, on).await
    }

    /// Whether `session` has feedback on, as stored (off when it cannot be
    /// read: nothing is sent on a guess).
    pub(crate) async fn feedback_on(&self, session: SessionId) -> bool {
        match self.store.feedback(session).await {
            Ok(stored) => stored.on,
            Err(error) => {
                tracing::warn!(%session, %error, "feedback could not be read");
                false
            }
        }
    }
}
