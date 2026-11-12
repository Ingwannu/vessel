//! What every session shares with its handles, built before its actor
//! runs: where its records start, its info watch, ring, channels and
//! lifecycle; then the actor spawned and the handle registered.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, PoisonError};

use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;
use tracing::Instrument as _;
use vessel_proto::{EventRecord, EventSeq, ModelCall, SessionInfo};

use super::{Created, SessionRuntime};
use crate::activity::Tracker;
use crate::conversation::Seen;
use crate::delivery::Desk;
use crate::handle::{Input, SessionHandle, Shared};
use crate::lifecycle::{Lifecycle, Rename};
use crate::marks::Marks;
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
}

impl Begin {
    /// A new session's beginning.
    pub(crate) fn new(info: SessionInfo, since: DateTime<Utc>) -> Self {
        Self {
            info,
            since,
            seen: Seen::default(),
            attention: None,
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

/// How many renames may wait for a session's actor; a caller waits for
/// room (each also waits for its own to be published).
const RENAME_CAPACITY: usize = 8;

impl SessionRuntime {
    /// Everything a session needs besides its process.
    pub(super) fn parts(
        &self,
        begin: Begin,
        notices: Vec<String>,
        input: Input,
        control: Option<mpsc::Sender<Control>>,
    ) -> Parts {
        let Begin {
            mut info,
            since,
            seen,
            attention,
        } = begin;
        // On the board from the start (starting is active), so no reader
        // of a live session ever sees it without a place.
        let activity = Tracker::new(since);
        info.board = Some(activity.board(since));
        let titled = info.title.is_some();
        let (model_calls, recorded) = mpsc::channel(MODEL_CALL_CAPACITY);
        let (renames, renaming) = mpsc::channel(RENAME_CAPACITY);
        let state = (seen, Marks::after(attention));
        let (info, kill, shared) = self.shared(info, input, control, (model_calls, renames), state);
        let (durable_sender, durable) = mpsc::channel(self.config.durable_capacity.get());
        Parts {
            life: Lifecycle {
                out: Publisher::new(Arc::clone(&shared), durable_sender, info, activity),
                stop: Stopper::new(kill, self.shutdown.clone()),
                notices,
                model_calls: recorded,
                renames: renaming,
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
        input: Input,
        control: Option<mpsc::Sender<Control>>,
        (model_calls, renames): (mpsc::Sender<ModelCall>, mpsc::Sender<Rename>),
        (seen, marks): (Seen, Marks),
    ) -> (watch::Sender<SessionInfo>, CancellationToken, Arc<Shared>) {
        let id = info.id;
        let next = info.next_seq;
        let (info, info_receiver) = watch::channel(info);
        let kill = self.shutdown.child_token();
        let shared = Arc::new(Shared {
            id,
            info: info_receiver,
            ring: Mutex::new(Ring::new(self.config.ring_capacity, next)),
            input,
            kill: kill.clone(),
            control,
            model_calls,
            ended: AtomicBool::new(false),
            renames,
            reported: Mutex::new(None),
            seen: Mutex::new(seen),
            // A new session has no marks; one started again takes the
            // store's once its row is reopened.
            marks: Mutex::new(marks),
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
        Created { handle, durable }
    }
}
