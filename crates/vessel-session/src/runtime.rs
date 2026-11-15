//! [`SessionRuntime`]: starts sessions and keeps their handles.
//!
//! Divided by what each part does: `launch` starts a new session of
//! either mode from its spec, `agent` starts and moves a structured
//! session's agent (and probes one), `parts` builds what every session
//! shares with its handles and registers it; this module holds the map of
//! handles and its lifetime.

mod agent;
pub use agent::ProbeChoices;
mod claims;
mod drivers;
mod launch;
mod parts;
mod preparation;
#[cfg(test)]
mod private_dir;

mod diagnostics;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Arc, PoisonError, RwLock};
use std::time::Duration;

use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::{EventRecord, SessionId, SessionInfo};

pub(crate) use agent::AgentCommand;
pub use launch::{ForkStart, LaunchOptions};
pub(crate) use parts::{ActorStart, Begin};

use crate::attachments::AttachmentLoader;
use crate::config::RuntimeConfig;
use crate::error::RemoveError;
use crate::handle::SessionHandle;

/// Runs every session of one node.
///
/// [Decision Log]
/// - Purpose: `get` and `list` are synchronous (the plan's R6 API), so the
///   set of sessions must be readable without a round trip to a task.
/// - Alternatives: a registry actor (every `get` becomes async, for a map
///   that changes only on create); a concurrent map crate (a dependency for
///   one map).
/// - Choice: a `std` `RwLock` over a map of handles. It is not a shared
///   mutable "god object": it only maps ids to handles, entries are added
///   once and removed once, all session state stays inside each session's
///   actor, and the lock is never held across `.await`.
/// - Lifetime: an ended session stays in the map (and keeps its ring, up to
///   `ring_capacity` records) until its owner calls [`remove`](Self::remove);
///   `list` reports it until then, as the plan requires. The node removes a
///   session once its log is persisted, after which the store serves it.
///   The runtime does not evict on its own: it cannot know when the store
///   has everything.
#[derive(Debug)]
pub struct SessionRuntime {
    config: RuntimeConfig,
    shutdown: CancellationToken,
    tasks: TaskTracker,
    sessions: RwLock<HashMap<SessionId, SessionHandle>>,
    /// The node's attachment loader (protocol 10), which every structured
    /// session asks for its prompts' attachments.
    attachments: Option<AttachmentLoader>,
    /// How long an idle structured session runs before its agent is
    /// parked, as the node now says (protocol 12, ADR 0038): every actor
    /// follows it, so a change applies to live sessions at once.
    park_after: watch::Sender<Option<Duration>>,
    /// Conversations sessions are starting on (protocol 17, `claims`).
    claims: Arc<claims::Claims>,
    /// Which protocol drives each agent's new sessions (protocol 18,
    /// `drivers`).
    drivers: drivers::Drivers,
}

/// A session that was just started.
#[derive(Debug)]
pub struct Created {
    /// The session.
    pub handle: SessionHandle,
    /// Every record, in order, exactly once. The node must drain it into the
    /// store. Bounded: if the node stops draining, the session applies
    /// backpressure (its output pump pauses) rather than dropping records.
    /// Closes after the final status record. The records are shared with
    /// the ring and live followers, not copies.
    pub durable: mpsc::Receiver<Arc<EventRecord>>,
}

impl SessionRuntime {
    /// A runtime whose sessions all stop when `shutdown` is cancelled.
    #[must_use]
    pub fn new(config: RuntimeConfig, shutdown: CancellationToken) -> Self {
        let (park_after, _) = watch::channel(config.park_after);
        Self {
            config,
            shutdown,
            tasks: TaskTracker::new(),
            sessions: RwLock::new(HashMap::new()),
            attachments: None,
            park_after,
            claims: Arc::new(claims::Claims::default()),
            drivers: drivers::Drivers::default(),
        }
    }

    /// How long an idle structured session runs before its agent is
    /// parked; `None` never parks (protocol 12).
    #[must_use]
    pub fn park_after(&self) -> Option<Duration> {
        *self.park_after.borrow()
    }

    /// Change [`park_after`](Self::park_after) for every structured
    /// session, live ones included: each re-arms its idle timer from its
    /// last activity, so one idle for longer than a new, shorter time is
    /// parked at once. Off disarms the timers; parked sessions stay parked
    /// until something wakes them.
    pub fn set_park_after(&self, after: Option<Duration>) {
        let _before = self.park_after.send_replace(after);
    }

    /// The setting, for a new actor to follow.
    pub(crate) fn park_setting(&self) -> watch::Receiver<Option<Duration>> {
        self.park_after.subscribe()
    }

    /// Structured sessions load their prompts' attachments through
    /// `loader` (protocol 10, ADR 0025). Without one, a prompt naming an
    /// attachment is refused.
    #[must_use]
    pub fn with_attachments(mut self, loader: AttachmentLoader) -> Self {
        self.attachments = Some(loader);
        self
    }

    /// A token that keeps [`shutdown`](Self::shutdown) waiting while a
    /// session is being started.
    pub(crate) fn tasks_token(&self) -> tokio_util::task::task_tracker::TaskTrackerToken {
        self.tasks.token()
    }

    /// Whether shutdown began: no session starts any more.
    pub(crate) fn is_shutting_down(&self) -> bool {
        self.shutdown.is_cancelled()
    }

    /// How long an agent's handshake may take.
    pub(crate) const fn handshake_timeout(&self) -> Duration {
        self.config.agent_handshake_timeout
    }

    /// The node's private directory a session's launches carry
    /// (`RuntimeConfig::private_dir`).
    pub(crate) fn private_dir(&self) -> Option<std::path::PathBuf> {
        self.config.private_dir.clone()
    }

    /// A session of this run, live or ended (until [`remove`](Self::remove)d).
    #[must_use]
    pub fn get(&self, id: SessionId) -> Option<SessionHandle> {
        self.sessions
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&id)
            .cloned()
    }

    /// The process ids of this run's live sessions' programs, each with
    /// its session: the roots of the trees whose listening ports the node
    /// reports (protocol 11, ADR 0031).
    #[must_use]
    pub fn pids(&self) -> Vec<(u32, SessionId)> {
        self.sessions
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .filter_map(|handle| Some((handle.pid()?, handle.id())))
            .collect()
    }

    /// Every session of this run, live and ended (until removed), newest
    /// first.
    #[must_use]
    pub fn list(&self) -> Vec<SessionInfo> {
        let mut infos: Vec<SessionInfo> = self
            .sessions
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .map(SessionHandle::info)
            .collect();
        infos.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        infos
    }

    /// Forget an ended session: the runtime drops its handle, so the
    /// session's ring, input path and remaining state are freed once the
    /// last other holder lets go. Returns the session's final info.
    ///
    /// Legal only once the session has ended, meaning its actor published
    /// the final status and closed the ring and the durable channel (no
    /// record can follow). The node calls it when its persistence task has
    /// drained the durable channel to its end, which implies that, or after
    /// [`SessionHandle::ended`] otherwise; from then on the session exists
    /// only in the store.
    ///
    /// Afterwards [`get`](Self::get) returns `None` and [`list`](Self::list)
    /// leaves the session out. Nothing else changes for anyone already
    /// holding it:
    /// - other `SessionHandle` clones stay valid: `info` returns the final
    ///   info, `send` fails with
    ///   [`SessionError::Ended`](crate::SessionError::Ended), `kill` does
    ///   nothing, and `follow` still serves the ring (with a receiver that
    ///   is already closed);
    /// - live followers already have every record up to the final status
    ///   and then see `Closed`, exactly as without the removal.
    ///
    /// # Errors
    /// [`RemoveError::NotFound`] for an unknown or already removed id, and
    /// [`RemoveError::Running`] while the session can still publish records
    /// (nothing is removed).
    pub fn remove(&self, id: SessionId) -> Result<SessionInfo, RemoveError> {
        // Checked and removed under one write lock, so a concurrent call
        // cannot remove it in between. Lock order: the map, then the
        // session's ring (nothing takes them the other way round).
        let removed = match self
            .sessions
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(id)
        {
            Entry::Occupied(entry) if entry.get().is_ended() => entry.remove(),
            Entry::Occupied(_) => return Err(RemoveError::Running(id)),
            Entry::Vacant(_) => return Err(RemoveError::NotFound(id)),
        };
        // Ended, so it held no conversation for the live check already.
        self.release(id);
        tracing::debug!(session = %id, "session removed from the runtime");
        Ok(removed.info())
    }

    /// Kill everything and wait until every session published its final
    /// status (`Interrupted` for sessions that were still running) and
    /// closed its durable channel. The node must keep draining durable
    /// channels while this runs.
    pub async fn shutdown(&self) {
        self.shutdown.cancel();
        let _ = self.tasks.close();
        self.tasks.wait().await;
    }
}
