//! What every session shares with its handles, built before its actor
//! runs: where its records start, its info watch, ring, channels and
//! lifecycle; then the actor spawned and the handle registered.

use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex, PoisonError};

use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;
use tracing::Instrument as _;
use vessel_agent::Driver;
use vessel_proc::ProcessLifetime;
use vessel_proto::{EventRecord, EventSeq, ModelCall, SessionInfo};

use super::{Created, SessionRuntime};
use crate::activity::Tracker;
use crate::conversation::Seen;
use crate::delivery::Desk;
use crate::forge::ForgeView;
use crate::handle::{Input, MetadataChange, SessionHandle, Shared};
use crate::kept_metadata::KeptMetadata;
use crate::lifecycle::Lifecycle;
use crate::moving::Control;
use crate::publish::Publisher;
use crate::ring::Ring;
use crate::stop::Stopper;

/// Where a session's records start: its info as its run begins (status
/// `starting`, `next_seq` the first record's number), when its activity
/// begins, and what its conversation's projection and attention point
/// start from. A new session starts from nothing; one started again from
/// what its stored log left (`again`).
pub(crate) struct Begin {
    pub(crate) info: SessionInfo,
    pub(crate) since: DateTime<Utc>,
    pub(crate) seen: Seen,
    pub(crate) attention: Option<EventSeq>,
    pub(crate) limits: crate::LimitAssociation,
}

impl Begin {
    /// A new session's beginning.
    pub(crate) fn new(info: SessionInfo, since: DateTime<Utc>) -> Self {
        Self {
            info,
            since,
            seen: Seen::default(),
            attention: None,
            limits: crate::LimitAssociation::Unavailable,
        }
    }
}

/// How a structured session's actor starts beyond its launch and its
/// records: a new session's from nothing; one started again with its
/// queue, its agent's replay dropped until it is ready, and a gate.
#[derive(Default)]
pub(crate) struct ActorStart {
    pub(crate) desk: Desk,
    pub(crate) replaying: bool,
    pub(crate) gate: Option<oneshot::Receiver<()>>,
    /// A record the log starts with after `starting` (a fork's `forked`,
    /// protocol 14). Boxed: a session event is large, and every start's
    /// future would carry its size.
    pub(crate) opening: Option<Box<vessel_proto::SessionEvent>>,
}

/// How many model calls may wait for a session's actor to record them; a
/// call that finds the queue full is dropped with a warning (the proxy is
/// never held up by the log).
const MODEL_CALL_CAPACITY: usize = 64;

/// The process-independent parts of a new session.
pub(super) struct Parts {
    pub(super) shared: Arc<Shared>,
    pub(super) life: Lifecycle,
    pub(super) durable: mpsc::Receiver<Arc<EventRecord>>,
}

/// How many metadata changes may wait for a session's actor; a caller waits for
/// room (each also waits for its own to be published).
const METADATA_CAPACITY: usize = 8;

/// The senders the shared state holds, for the actor's receivers: model
/// calls, metadata, and the node's view of the session's pull request.
type Channels = (
    mpsc::Sender<ModelCall>,
    mpsc::Sender<MetadataChange>,
    watch::Sender<Option<Arc<ForgeView>>>,
);

impl SessionRuntime {
    /// Everything a session needs besides its process.
    pub(super) fn parts(
        &self,
        begin: Begin,
        notices: Vec<String>,
        (input, process): (Input, ProcessLifetime),
        control: (Option<mpsc::Sender<Control>>, Driver),
    ) -> Parts {
        let Begin {
            mut info,
            since,
            seen,
            attention,
            limits,
        } = begin;
        // On the board from the start (starting is active), so no reader
        // of a live session ever sees it without a place.
        let activity = Tracker::new(since);
        info.board = Some(activity.board(since));
        let titled = info.title.is_some();
        let (model_calls, recorded) = mpsc::channel(MODEL_CALL_CAPACITY);
        let (metadata, metadata_changes) = mpsc::channel(METADATA_CAPACITY);
        let (forge, forge_view) = watch::channel(None);
        let state = (seen, KeptMetadata::after(attention, info.detach_receipt));
        let channels = (model_calls, metadata, forge);
        let (info, kill, shared) = self.shared(info, (input, process), control, channels, state);
        let _previous = shared.limits.send_replace(limits);
        let (durable_sender, durable) = mpsc::channel(self.config.durable_capacity.get());
        Parts {
            life: Lifecycle {
                out: Publisher::new(Arc::clone(&shared), durable_sender, info, activity),
                stop: Stopper::new(kill, self.shutdown.clone()),
                notices,
                model_calls: recorded,
                metadata: metadata_changes,
                forge: forge_view,
                titled,
            },
            shared,
            durable,
        }
    }

    /// Initial info and the state shared by the actor and its handles.
    fn shared(
        &self,
        info: SessionInfo,
        (input, process): (Input, ProcessLifetime),
        (control, driver): (Option<mpsc::Sender<Control>>, Driver),
        (model_calls, metadata, forge): Channels,
        (seen, marks): (Seen, KeptMetadata),
    ) -> (watch::Sender<SessionInfo>, CancellationToken, Arc<Shared>) {
        let id = info.id;
        let next = info.next_seq;
        let (info, info_receiver) = watch::channel(info);
        let kill = self.shutdown.child_token();
        let shared = Arc::new(Shared {
            id,
            limits: watch::channel(crate::LimitAssociation::Unavailable).0,
            info: info_receiver,
            ring: Mutex::new(Ring::new(self.config.ring_capacity, next)),
            input,
            kill: kill.clone(),
            control,
            driver,
            model_calls,
            ended: AtomicBool::new(false),
            metadata,
            reported: Mutex::new(None),
            seen: Mutex::new(seen),
            // A new session has no marks; one started again takes the
            // store's once its row is reopened.
            kept_metadata: Mutex::new(marks),
            forge,
            pid: AtomicU32::new(process.pid().get()),
            process: Mutex::new(Some(process)),
        });
        (info, kill, shared)
    }

    /// Spawn the actor, register the handle, and hand both ends out.
    pub(crate) fn start(
        &self,
        shared: Arc<Shared>,
        actor: impl Future<Output = ()> + Send + 'static,
        durable: mpsc::Receiver<Arc<EventRecord>>,
    ) -> Created {
        let handle = SessionHandle(shared);
        // The actor can publish as soon as it is spawned. Its row must start
        // from this earlier map/cursor pair even if persistence starts later.
        let initial = Box::new(handle.info());
        let id = handle.id();
        let span = tracing::info_span!("session", session = %id);
        drop(self.tasks.spawn(actor.instrument(span)));
        drop(
            self.sessions
                .write()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(id, handle.clone()),
        );
        tracing::info!(session = %id, harness = %handle.info().spec.harness, "session started");
        Created {
            initial,
            handle,
            durable,
        }
    }
}
