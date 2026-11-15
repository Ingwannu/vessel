//! The node's sessions: live ones in the runtime, and every one ever
//! stored, including those from earlier runs.
//!
//! A session created in this run is "live" here while the runtime has it:
//! from its creation until its whole log, final status included, is
//! committed. Then it is handed over: removed from the runtime (which frees
//! its ring) and from the durability table, and served from the store like
//! a session of an earlier run. Stored sessions can be listed, read and
//! followed, and refuse input with `Conflict`. Handles and subscriptions
//! taken before the hand-over keep working (the runtime guarantees that),
//! and every record they could still need is in the store by then.
//!
//! The only state this type adds to the runtime's is each live session's
//! durability watch, kept in a small id-keyed table that is written once per
//! session and otherwise only read (lock held for a lookup, never across an
//! `.await`).
//!
//! That table is also what makes a runtime session visible. The runtime
//! registers a session before `create` returns, and the store row is
//! written after; a session is listed and found only once it has a table
//! entry, which is inserted before the store row is awaited. So a session
//! is never listed without being findable, and never found as a stored
//! (ended) session while it runs.

mod creation;
mod mutation_time;
mod subagents;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, PoisonError, RwLock};

use tokio::sync::{mpsc, watch};
use tokio_util::task::TaskTracker;
use vessel_harness::{ModelProxy, Profile};
use vessel_proto::{
    EventRecord, EventSeq, HarnessRef, OptionChoice, ProfileName, SessionId, SessionInfo,
    SessionOption, SessionTitle,
};
use vessel_session::{ProfileMove, SessionHandle, SessionRuntime};
use vessel_store::Store;

use crate::agents::Agents;
use crate::error::ServiceError;
use crate::models::Models;
use crate::persist::{self, Durability, Persisted};
use crate::workspace::Occupant;

/// Where a session was found.
#[expect(
    clippy::large_enum_variant,
    reason = "returned once per lookup and matched at once, never queued or \
              stored; boxing would add an allocation to every lookup"
)]
pub(crate) enum Found {
    /// Created in this run and not yet handed over to the store.
    Live {
        handle: SessionHandle,
        durability: watch::Receiver<Durability>,
    },
    /// Ended, and served from the store.
    Stored(SessionInfo),
}

/// Live and stored sessions behind one interface.
#[derive(Debug)]
pub(crate) struct Sessions {
    runtime: Arc<SessionRuntime>,
    store: Store,
    durability: RwLock<HashMap<SessionId, watch::Receiver<Durability>>>,
    /// Session creations and persist tasks; shutdown waits for both.
    tasks: TaskTracker,
    /// Sessions' agents: a session's credential is retired when it ends.
    agents: Arc<Agents>,
    /// Model routes: a session's model token is retired when it ends.
    models: Arc<Models>,
    /// Serializes marks, from the store's write to the handle's copy
    /// (`marks`), and a session's start again from its becoming visible to
    /// its handle taking the store's marks (`again`).
    marking: tokio::sync::Mutex<()>,
    /// Sessions being started again (`again`), one call each.
    claims: again::Claims,
    /// Each session's end, once its log is complete (protocol 11): what an
    /// agent's schedules end with (`automation`). Lossy by design: a
    /// listener that falls behind finds the agent gone at the schedule's
    /// next run instead.
    ended: tokio::sync::broadcast::Sender<SessionId>,
}

/// Ends a listener may fall behind by before it misses some.
const ENDED: usize = 64;

impl Sessions {
    pub(crate) fn limit_association(&self, id: SessionId) -> vessel_session::LimitAssociation {
        self.runtime
            .get(id)
            .map_or(vessel_session::LimitAssociation::Unavailable, |handle| {
                handle.limit_association()
            })
    }

    pub(crate) fn new(
        runtime: Arc<SessionRuntime>,
        store: Store,
        tasks: TaskTracker,
        agents: Arc<Agents>,
        models: Arc<Models>,
    ) -> Self {
        Self {
            runtime,
            store,
            durability: RwLock::new(HashMap::new()),
            tasks,
            agents,
            models,
            marking: tokio::sync::Mutex::new(()),
            claims: again::Claims::default(),
            ended: tokio::sync::broadcast::Sender::new(ENDED),
        }
    }

    /// The ends of sessions from now on (see the field).
    pub(crate) fn ends(&self) -> tokio::sync::broadcast::Receiver<SessionId> {
        self.ended.subscribe()
    }

    pub(crate) const fn store(&self) -> &Store {
        &self.store
    }

    /// Make a session the runtime has just started (or started again)
    /// visible: its agent's and its model token's places take its handle,
    /// and its durability entry says every record before `stored` is
    /// committed (none for a new session, its stored log for one started
    /// again).
    fn track(&self, handle: &SessionHandle, stored: EventSeq) -> watch::Sender<Durability> {
        let id = handle.id();
        self.agents.started(id, handle.clone());
        self.models.started(id, handle.clone());
        let (durability, watching) = watch::channel(Durability::Through(stored));
        let previous = self
            .durability
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, watching);
        if previous.is_some() {
            tracing::error!(session = %id, "session id created twice");
        }
        durability
    }

    /// Drain the session's records into the store, then hand it over.
    fn persist(
        self: &Arc<Self>,
        handle: SessionHandle,
        durable: mpsc::Receiver<Arc<EventRecord>>,
        durability: watch::Sender<Durability>,
    ) {
        let sessions = Arc::clone(self);
        let store = self.store.clone();
        drop(self.tasks.spawn(async move {
            let persisted = persist::run(store, handle.clone(), durable, durability).await;
            sessions.hand_over(handle, persisted).await;
        }));
    }

    /// The session's log is complete in the store (or never will be): let
    /// the store serve it, and free what the runtime kept for it.
    ///
    /// A complete log means the durable channel was drained to its end, so
    /// the session has ended and `remove` accepts it. Otherwise persistence
    /// stopped early and killed the session; it is removed once it has
    /// ended.
    ///
    /// The agent's credential dies with the session (ADR 0013), and so
    /// does its model token (ADR 0015): both already refuse once its end is
    /// visible (`SessionHandle::has_ended`), and are removed here, before
    /// the store serves the session, so a run that starts it again
    /// (`again`) never has its own places taken by this run's end.
    async fn hand_over(&self, handle: SessionHandle, persisted: Persisted) {
        let id = handle.id();
        if persisted == Persisted::Incomplete {
            let _final = handle.ended().await;
        }
        self.agents.retire(id);
        self.models.retire(id);
        if let Err(error) = self.runtime.remove(id) {
            tracing::warn!(session = %id, %error, "cannot remove a finished session from the runtime");
        }
        self.forget(id);
        // End listeners recheck find(). Publish only after both live
        // entries are gone, or they can consume the only wake while still
        // finding Live and wait for an end that will never be sent again.
        let _listeners = self.ended.send(id);
    }

    /// Drop a session's durability entry; it is then found in the store.
    fn forget(&self, id: SessionId) {
        drop(
            self.durability
                .write()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&id),
        );
    }

    pub(crate) async fn find(&self, id: SessionId) -> Result<Found, ServiceError> {
        let durability = self
            .durability
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&id)
            .cloned();
        if let Some(durability) = durability
            && let Some(handle) = self.runtime.get(id)
        {
            return Ok(Found::Live { handle, durability });
        }
        match self.store.session(id).await? {
            Some(info) => Ok(Found::Stored(info)),
            None => Err(ServiceError::NotFound(id)),
        }
    }

    /// Every session, newest first. Live sessions report the runtime's view,
    /// which is ahead of the store's; runtime sessions without a table entry
    /// (still being created, or failed to store) are left out, so every
    /// listed session can be found.
    pub(crate) async fn list(&self) -> Result<Vec<SessionInfo>, ServiceError> {
        let mut all: HashMap<SessionId, SessionInfo> = self
            .store
            .sessions()
            .await?
            .into_iter()
            .map(|info| (info.id, info))
            .collect();
        let visible: Vec<SessionInfo> = {
            let durability = self
                .durability
                .read()
                .unwrap_or_else(PoisonError::into_inner);
            self.runtime
                .list()
                .into_iter()
                .filter(|info| durability.contains_key(&info.id))
                .collect()
        };
        for info in visible {
            drop(all.insert(info.id, info));
        }
        let mut list: Vec<SessionInfo> = all.into_values().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(list)
    }

    /// The agent and profile of every live session (`None`: its agent's
    /// default): what `agent.status` does not check beside (protocol 11).
    pub(crate) fn live_profiles(&self) -> Vec<(HarnessRef, Option<ProfileName>)> {
        self.runtime
            .list()
            .into_iter()
            .filter(|info| info.status.is_live())
            .map(|info| (info.spec.harness, info.profile))
            .collect()
    }

    /// The live sessions and the folders they run in (a session without
    /// one runs in the node's home, which no worktree is in): what keeps a
    /// worktree from being removed (protocol 10).
    pub(crate) fn live_folders(&self) -> Vec<(Occupant, PathBuf)> {
        self.runtime
            .list()
            .into_iter()
            .filter(|info| info.status.is_live())
            .filter_map(|info| Some((Occupant::Session(info.id), info.spec.cwd?)))
            .collect()
    }

    /// The runtime's handle of `id` while the runtime holds it (protocol
    /// 11: an agent's terminals end with its session).
    pub(crate) fn runtime_handle(&self, id: SessionId) -> Option<SessionHandle> {
        self.runtime.get(id)
    }

    /// The process ids of the live sessions' programs, for the ports their
    /// trees listen on (protocol 11, ADR 0031).
    pub(crate) fn live_pids(&self) -> Vec<(u32, SessionId)> {
        self.runtime.pids()
    }

    /// Whether `id` is gone for good: the store no longer has it (deleted,
    /// or dropped by retention), and the runtime has nothing of it but its
    /// end (protocol 10's tidying). Retention prunes a session in the
    /// commit of its final record, before the runtime lets it go, so an
    /// ended session the runtime still holds counts; a session is the
    /// runtime's, live, from its start and the store's from its first row,
    /// so one being created never does. A store that cannot answer says it
    /// is not gone, so nothing is tidied on a guess.
    pub(crate) async fn gone(&self, id: SessionId) -> bool {
        if self
            .runtime
            .get(id)
            .is_some_and(|handle| !handle.has_ended())
        {
            return false;
        }
        match self.store.session(id).await {
            Ok(found) => found.is_none(),
            Err(error) => {
                tracing::warn!(session = %id, %error, "could not tell whether a session is gone");
                false
            }
        }
    }

    /// The live session `id`, when the runtime has it.
    pub(crate) fn live(&self, id: SessionId) -> Option<SessionHandle> {
        self.runtime.get(id).filter(|handle| !handle.has_ended())
    }

    pub(crate) async fn get(&self, id: SessionId) -> Result<SessionInfo, ServiceError> {
        Ok(match self.find(id).await? {
            Found::Live { handle, .. } => handle.info(),
            Found::Stored(info) => info,
        })
    }

    /// The live session `id`, for a call that acts on its agent (protocol
    /// 12's park and wake); an ended one is `Ended`.
    pub(crate) async fn acting_on(&self, id: SessionId) -> Result<SessionHandle, ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => Ok(handle),
            Found::Stored(_) => Err(ServiceError::Ended(id)),
        }
    }

    /// Move a live session to another profile (`profiles::moving` plans it).
    pub(crate) async fn move_profile(
        &self,
        id: SessionId,
        plan: impl AsyncFnOnce(&SessionInfo) -> Result<ProfileMove, ServiceError>,
    ) -> Result<SessionInfo, ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => {
                let request = plan(&handle.info()).await?;
                Ok(self.runtime.move_profile(id, request).await?)
            }
            Found::Stored(_) => Err(ServiceError::Ended(id)),
        }
    }

    /// A live session's agent options as its agent last reported them
    /// (protocol 9). An ended session has no agent to ask: its log's last
    /// `options_changed` is what it ran with.
    pub(crate) async fn options(&self, id: SessionId) -> Result<Vec<SessionOption>, ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => Ok(handle.options().await?),
            Found::Stored(_) => Err(ServiceError::Ended(id)),
        }
    }

    /// Change one of a live session's agent options (protocol 9).
    pub(crate) async fn set_option(
        &self,
        id: SessionId,
        choice: OptionChoice,
    ) -> Result<Vec<SessionOption>, ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => Ok(handle.set_option(choice).await?),
            Found::Stored(_) => Err(ServiceError::Ended(id)),
        }
    }

    /// Give a live session a title (protocol 9). An ended session's log is
    /// closed, and its title is the last one it had.
    pub(crate) async fn rename(
        &self,
        id: SessionId,
        title: SessionTitle,
    ) -> Result<SessionInfo, ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => Ok(handle.rename(title).await?),
            Found::Stored(_) => Err(ServiceError::Ended(id)),
        }
    }

    /// The agent options a session runs (or ran) with, as choices
    /// (protocol 9): its last options report's, from the runtime or the
    /// store; before any report, its spec's own.
    pub(crate) async fn choices(&self, id: SessionId) -> Result<Vec<OptionChoice>, ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => Ok(handle.choices()),
            Found::Stored(info) => Ok(match self.store.choices(id).await? {
                Some(choices) => choices,
                None => info.spec.mode.options().to_vec(),
            }),
        }
    }

    /// What `harness`'s agent offers (`SessionRuntime::probe`).
    pub(crate) async fn probe(
        &self,
        harness: HarnessRef,
        profile: Profile,
        proxy: Option<ModelProxy>,
        dir: PathBuf,
    ) -> Result<vessel_session::Probed, ServiceError> {
        Ok(self
            .runtime
            .probe(harness, Some(profile), proxy, dir)
            .await?)
    }

    /// A page of `harness`'s own conversations (`SessionRuntime::history`).
    pub(crate) async fn history(
        &self,
        harness: HarnessRef,
        launch: (Option<Profile>, Option<ModelProxy>),
        dir: PathBuf,
        folder: Option<&std::path::Path>,
        from: vessel_agent::HistoryFrom,
    ) -> Result<vessel_agent::HistoryPage, ServiceError> {
        Ok(self
            .runtime
            .history(harness, launch, dir, folder, from)
            .await?)
    }

    /// Stop a session. Killing one that already ended is a no-op.
    pub(crate) async fn kill(&self, id: SessionId) -> Result<(), ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => handle.kill(),
            Found::Stored(_) => {}
        }
        Ok(())
    }
}

// Protocol 10's marks and deletion, and starting an ended session again;
// the conversation's input; reading the log.
pub(crate) mod again;
mod conversation;
mod discovery;
mod marks;
mod read;
mod side_questions;

#[cfg(test)]
mod tests;
