//! Bounded stream membership; each admitted id has one serial generation worker.
//!
//! Map locks cover only admission/lookup/identity cleanup, never preparation or
//! teardown. Replacement metadata has one mailbox slot; a full slot refuses
//! before changing either the current generation or its pending replacement.

use super::{
    commands::Handle,
    control::Control,
    input::Input,
    prepare::{Factory, PrepareError},
    quota::{ConnectionBudget, NodeBudget, QuotaError},
    stream_error::StreamError,
    updates::Event,
};
use crate::connection_life::{ConnectionLife, Owner};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, Notify, mpsc, oneshot, watch};
use tokio_util::sync::{CancellationToken, DropGuard};
use tokio_util::task::TaskTracker;
use vessel_proto::dictation::{
    ChunkSeq, DictationId, DictationPcm, Pcm16Mono, SetDictation, WithinMs,
};
use vessel_rpc::Outbound;

#[cfg(test)]
pub(super) mod test_fixture;
#[cfg(test)]
mod tests;
mod worker;
const REPLACEMENT_MAILBOX: usize = 1;
type Result<T> = std::result::Result<T, Arc<RegistryError>>;

/// Scope checking remains at dispatch; this bundle cannot come from a wire id.
#[derive(Clone)]
pub(super) struct Caller {
    owner: Owner,
    closed: CancellationToken,
    budget: ConnectionBudget,
    outbound: Outbound,
}
impl Caller {
    /// Retain observers and quota only, never the real context's lifetime guard.
    pub(super) fn new(life: &ConnectionLife, budget: ConnectionBudget, outbound: Outbound) -> Self {
        Self {
            owner: life.owner(),
            closed: life.closed(),
            budget,
            outbound,
        }
    }
}
#[derive(Clone)]
/// Read current overrides after old teardown; test snapshots can force that boundary.
pub(super) enum Snapshot {
    Saved(Arc<crate::settings::Settings>),
    #[cfg(test)]
    Given(Arc<tokio::sync::RwLock<SetDictation>>),
}
impl Snapshot {
    async fn current(&self) -> SetDictation {
        match self {
            Self::Saved(settings) => settings.dictation_snapshot().await,
            #[cfg(test)]
            Self::Given(settings) => settings.read().await.clone(),
        }
    }
}
#[derive(thiserror::Error)]
/// Preserve private causes until one fixed service-boundary conversion.
pub(super) enum RegistryError {
    #[error("the dictation id belongs to another live connection")]
    Taken,
    #[error("this connection does not own the dictation stream")]
    Forbidden,
    #[error("the dictation stream has a pending preparation or replacement")]
    Conflict,
    #[error("the dictation replacement mailbox is occupied or closed")]
    Mailbox(#[source] mpsc::error::TrySendError<()>),
    #[error("the dictation stream no longer exists")]
    Gone,
    #[error("the dictation reservation limit was reached")]
    Quota(#[source] QuotaError),
    #[error("the dictation start format was refused")]
    Input(#[source] super::input::InputError),
    #[error("the dictation provider could not be prepared")]
    Prepare(#[source] PrepareError),
    #[error("the dictation stream did not accept the call")]
    Stream(#[source] Arc<StreamError>),
    #[error("the dictation worker did not finish")]
    Worker(#[source] tokio::task::JoinError),
    #[error("the dictation start closed without its reply")]
    Reply(#[source] oneshot::error::RecvError),
}
impl std::fmt::Debug for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

struct Start {
    format: Pcm16Mono,
    reply: oneshot::Sender<Result<()>>,
}
#[derive(Clone)]
enum Phase {
    Preparing(Control<StreamError>),
    Live(Handle),
    Retiring(Control<StreamError>),
}
impl Phase {
    fn control(&self) -> Control<StreamError> {
        match self {
            Self::Preparing(control) | Self::Retiring(control) => control.clone(),
            Self::Live(handle) => handle.control.clone(),
        }
    }
}
struct Entry {
    owner: Owner,
    starts: mpsc::Sender<Start>,
    changed: Notify,
    phase: watch::Sender<Phase>,
    stop: CancellationToken,
    finished: CancellationToken,
}
struct State {
    entries: Mutex<HashMap<DictationId, Arc<Entry>>>,
    budget: NodeBudget,
    factory: Arc<Factory>,
    snapshot: Snapshot,
    closed: CancellationToken,
    tasks: TaskTracker,
}
/// Service handles keep the stop guard; workers hold only a weak registry reference.
#[derive(Clone)]
pub(super) struct Registry {
    state: Arc<State>,
    _close_on_drop: Arc<DropGuard>,
}
impl Registry {
    /// Share admission and preparation once per node service, below node shutdown.
    pub(super) fn new(factory: Factory, snapshot: Snapshot, node: &CancellationToken) -> Self {
        let closed = node.child_token();
        Self {
            state: Arc::new(State {
                entries: Mutex::new(HashMap::new()),
                budget: NodeBudget::new(),
                factory: Arc::new(factory),
                snapshot,
                closed: closed.clone(),
                tasks: TaskTracker::new(),
            }),
            _close_on_drop: Arc::new(closed.drop_guard()),
        }
    }
    /// Admit bounded metadata before replacing anything; preparation stays off the map lock.
    pub(super) async fn start(
        &self,
        caller: &Caller,
        id: DictationId,
        format: Pcm16Mono,
    ) -> Result<()> {
        let _shape =
            Input::new(format.rate()).map_err(|source| Arc::new(RegistryError::Input(source)))?;
        if caller.closed.is_cancelled() || self.state.closed.is_cancelled() {
            return Err(Arc::new(RegistryError::Gone));
        }
        let (reply, response) = oneshot::channel();
        let command = Start { format, reply };
        {
            let mut entries = self.state.entries.lock().await;
            if self.state.closed.is_cancelled() {
                return Err(Arc::new(RegistryError::Gone));
            }
            entries.retain(|_, entry| !entry.finished.is_cancelled());
            self.state.admit(&mut entries, caller, id, command)?;
        }
        response
            .await
            .map_err(|source| Arc::new(RegistryError::Reply(source)))?
    }
    async fn entry(&self, caller: &Caller, id: &DictationId) -> Result<Arc<Entry>> {
        let entries = self.state.entries.lock().await;
        let entry = entries
            .get(id)
            .ok_or_else(|| Arc::new(RegistryError::Gone))?;
        if entry.owner != caller.owner {
            return Err(Arc::new(RegistryError::Forbidden));
        }
        if entry.finished.is_cancelled() {
            return Err(Arc::new(RegistryError::Gone));
        }
        Ok(Arc::clone(entry))
    }
    async fn handle(&self, caller: &Caller, id: &DictationId) -> Result<Handle> {
        let entry = self.entry(caller, id).await?;
        let phase = entry.phase.borrow().clone();
        match phase {
            Phase::Live(handle) if !handle.closed.is_cancelled() => Ok(handle),
            Phase::Live(_) => Err(Arc::new(RegistryError::Gone)),
            Phase::Preparing(_) | Phase::Retiring(_) => Err(Arc::new(RegistryError::Conflict)),
        }
    }
    /// Resolve ownership before forwarding validated audio to its current actor.
    pub(super) async fn chunk(
        &self,
        caller: &Caller,
        id: &DictationId,
        seq: ChunkSeq,
        pcm: DictationPcm,
    ) -> Result<Option<ChunkSeq>> {
        self.handle(caller, id)
            .await?
            .chunk(seq, pcm)
            .await
            .map_err(|source| Arc::new(RegistryError::Stream(source)))
    }
    /// Keep finish sequencing and its frozen deadline with the exclusive actor.
    pub(super) async fn finish(
        &self,
        caller: &Caller,
        id: &DictationId,
        last: ChunkSeq,
    ) -> Result<WithinMs> {
        self.handle(caller, id)
            .await?
            .finish(last)
            .await
            .map_err(|source| Arc::new(RegistryError::Stream(source)))
    }
    /// Ownership precedes the actor's lossless single-waiter update delivery.
    pub(super) async fn next(
        &self,
        caller: &Caller,
        id: &DictationId,
    ) -> Result<Event<Arc<StreamError>>> {
        self.handle(caller, id)
            .await?
            .next()
            .await
            .map_err(|source| Arc::new(RegistryError::Stream(source)))
    }
    /// Acknowledge after actual retirement; an already absent id is cancelled too.
    pub(super) async fn cancel(&self, caller: &Caller, id: &DictationId) -> Result<()> {
        let entry = match self.entry(caller, id).await {
            Ok(entry) => entry,
            Err(error) if matches!(error.as_ref(), RegistryError::Gone) => return Ok(()),
            Err(error) => return Err(error),
        };
        let _first = entry.phase.borrow().control().stop(
            Arc::new(StreamError::Cancelled),
            super::control::Retention::Forget,
        );
        entry.stop.cancel();
        entry.finished.cancelled().await;
        Ok(())
    }
    /// Observe service/node stop without retaining the service's drop guard.
    pub(super) fn track_shutdown(&self, tasks: &TaskTracker) {
        let state = Arc::downgrade(&self.state);
        let closed = self.state.closed.clone();
        let workers = self.state.tasks.clone();
        drop(tasks.spawn(async move {
            closed.cancelled().await;
            if let Some(state) = state.upgrade() {
                // Share admission's barrier before the close/wait boundary.
                let _admission = state.entries.lock().await;
                let _first_close = workers.close();
            } else {
                let _first_close = workers.close();
            }
            workers.wait().await;
        }));
    }

    /// Explicit fixture shutdown joins the same real preparation and retirement.
    #[cfg(test)]
    pub(super) async fn shutdown(&self) {
        self.state.closed.cancel();
        {
            // No admission can register a worker after the close/wait boundary.
            let _admission = self.state.entries.lock().await;
            let _first_close = self.state.tasks.close();
        }
        self.state.tasks.wait().await;
    }
}

fn remove_current(
    entries: &mut HashMap<DictationId, Arc<Entry>>,
    id: &DictationId,
    expected: &Arc<Entry>,
) {
    if entries
        .get(id)
        .is_some_and(|current| Arc::ptr_eq(current, expected))
    {
        let _removed = entries.remove(id);
    }
}

impl State {
    fn admit(
        self: &Arc<Self>,
        entries: &mut HashMap<DictationId, Arc<Entry>>,
        caller: &Caller,
        id: DictationId,
        command: Start,
    ) -> Result<()> {
        if let Some(entry) = entries.get(&id) {
            if entry.owner != caller.owner {
                return Err(Arc::new(RegistryError::Taken));
            }
            if entry.stop.is_cancelled() {
                return Err(Arc::new(RegistryError::Conflict));
            }
            let slot = entry
                .starts
                .try_reserve()
                .map_err(|source| Arc::new(RegistryError::Mailbox(source)))?;
            slot.send(command);
            entry.changed.notify_one();
        } else {
            let lease = self
                .budget
                .take(&caller.budget)
                .map_err(|source| Arc::new(RegistryError::Quota(source)))?;
            let control = Control::new(&caller.closed);
            let (phase, _receiver) = watch::channel(Phase::Preparing(control.clone()));
            let (starts, receiver) = mpsc::channel(REPLACEMENT_MAILBOX);
            let entry = Arc::new(Entry {
                owner: caller.owner.clone(),
                starts,
                changed: Notify::new(),
                phase,
                stop: CancellationToken::new(),
                finished: CancellationToken::new(),
            });
            let _previous = entries.insert(id.clone(), Arc::clone(&entry));
            worker::spawn(
                self,
                worker::Launch {
                    id,
                    entry,
                    caller: caller.clone(),
                    lease,
                    starts: receiver,
                    command,
                    control,
                },
            );
        }
        Ok(())
    }
}
