//! One ordered lane for public records and node-only commit barriers.
//!
//! [Decision Log]
//! - Purpose: a person's acknowledged stop must survive a daemon kill and
//!   must not race a later old-generation turn into rearming recovery.
//! - Constraint: public records already have a bounded persistence lane;
//!   a second checkpoint channel would lose its ordering against them.
//! - Choice: typed internal entries on that same lane, with actual commit
//!   acknowledgements. Only events receive public sequence numbers.
//! - Cost: a checkpoint-enabled consumer must supply the real writer; a
//!   records-only consumer refuses checkpoints rather than inventing an ACK.
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tokio::sync::{mpsc, oneshot};
use vessel_proto::restart::{RestartCheckpoint, RestartGeneration};
use vessel_proto::{AgentEvent, EventRecord, EventSeq, SessionEvent, WireError};

/// A persistence item. Checkpoints never acquire public event sequence numbers.
#[derive(Debug)]
pub enum JournalEntry {
    /// The ordinary record, shared with the ring and subscribers.
    Event(Arc<EventRecord>),
    /// A barrier that only the node's real store may acknowledge successfully.
    Checkpoint {
        /// Durable change, ordered after all preceding records.
        change: RestartCheckpoint,
        /// Whether it committed and still matched the session generation.
        reply: oneshot::Sender<Result<bool, JournalError>>,
    },
}
impl JournalEntry {
    /// Extract a record for a runtime that did not enable restart checkpoints.
    /// Returning the checkpoint forces such a consumer to handle/refuse it;
    /// silently acknowledging an unpersisted barrier would lose stop intent.
    ///
    /// # Errors
    /// This item is an internal checkpoint, not an event.
    pub fn into_event(self) -> Result<Arc<EventRecord>, Self> {
        match self {
            Self::Event(record) => Ok(record),
            item @ Self::Checkpoint { .. } => Err(item),
        }
    }
}
/// Why an ordered persistence barrier could not be satisfied.
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    /// The actor or persistence receiver has ended.
    #[error("session persistence is no longer available")]
    Unavailable,
    /// The bounded channel closed before admitting the checkpoint.
    #[error("session persistence channel closed")]
    Closed(#[source] mpsc::error::SendError<()>),
    /// No commit acknowledgement remained.
    #[error("session persistence ended before acknowledging the checkpoint")]
    Reply(#[source] oneshot::error::RecvError),
    /// Typed store failure mapped once at the node boundary.
    #[error("cannot commit session recovery state: {0}")]
    Commit(#[source] WireError),
    /// The stop counter cannot wrap or be durably represented.
    #[error("session recovery generation is exhausted")]
    Exhausted,
    /// A restored generation must be installed before any record is published.
    #[error("session recovery generation cannot change after publication")]
    AlreadyVisible,
}
#[derive(Debug)]
pub(crate) struct Control {
    enabled: bool,
    state: Mutex<State>,
}
#[derive(Debug)]
pub(crate) struct State {
    generation: RestartGeneration,
    turn: Option<EventSeq>,
    sender: Option<mpsc::Sender<JournalEntry>>,
    published: bool,
    parked: bool,
}
impl State {
    pub(crate) fn observe(&mut self, record: &EventRecord) {
        self.published = true;
        match &record.event {
            SessionEvent::Agent {
                event: AgentEvent::TurnStarted { .. },
            } => self.turn = Some(record.seq),
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded { .. } | AgentEvent::TurnFailed { .. },
            }
            | SessionEvent::Rewound { .. } => self.turn = None,
            SessionEvent::Parked { .. } => {
                self.parked = true;
                self.turn = None;
            }
            SessionEvent::Woken => self.parked = false,
            SessionEvent::Status { status } if !status.is_live() => {
                self.turn = None;
                self.sender = None;
            }
            SessionEvent::Status { .. }
            | SessionEvent::Agent { .. }
            | SessionEvent::Terminal(_)
            | SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            | SessionEvent::Forked { .. } => {}
        }
    }
}
impl Control {
    pub(crate) fn new(enabled: bool, sender: mpsc::Sender<JournalEntry>) -> Self {
        Self {
            enabled,
            state: Mutex::new(State {
                generation: RestartGeneration::ZERO,
                turn: None,
                sender: Some(sender),
                published: false,
                parked: false,
            }),
        }
    }
    pub(crate) fn publication(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }
    pub(crate) fn close(&self) {
        self.publication().sender = None;
    }
    pub(crate) fn shutdown_snapshot(&self) -> RestartCheckpoint {
        let state = self.publication();
        RestartCheckpoint::Shutdown {
            generation: state.generation,
            turn: state.turn,
            parked: state.parked,
        }
    }
    pub(crate) fn generation(&self) -> RestartGeneration {
        self.publication().generation
    }
    pub(crate) fn initialise(&self, generation: RestartGeneration) -> Result<(), JournalError> {
        let mut state = self.publication();
        if state.published {
            return Err(JournalError::AlreadyVisible);
        }
        state.generation = generation;
        Ok(())
    }
    fn sender(&self) -> Result<mpsc::Sender<JournalEntry>, JournalError> {
        self.publication()
            .sender
            .clone()
            .ok_or(JournalError::Unavailable)
    }
    pub(crate) async fn checkpoint(&self, change: RestartCheckpoint) -> Result<bool, JournalError> {
        if !self.enabled {
            return Ok(true);
        }
        let permit = self
            .sender()?
            .reserve_owned()
            .await
            .map_err(JournalError::Closed)?;
        let (reply, receive) = oneshot::channel();
        drop(permit.send(JournalEntry::Checkpoint { change, reply }));
        receive.await.map_err(JournalError::Reply)?
    }
    pub(crate) async fn suppress(
        &self,
        turn: Option<EventSeq>,
    ) -> Result<Option<RestartGeneration>, JournalError> {
        if !self.enabled {
            return Ok(None);
        }
        let permit = self
            .sender()?
            .reserve_owned()
            .await
            .map_err(JournalError::Closed)?;
        let (reply, receive) = oneshot::channel();
        let generation = {
            let mut state = self.publication();
            if state.sender.is_none() {
                return Err(JournalError::Unavailable);
            }
            if turn.is_some() && turn != state.turn {
                return Ok(None);
            }
            let generation = state.generation.next().ok_or(JournalError::Exhausted)?;
            state.generation = generation;
            // Publication uses this same lock: the observed turn and its
            // records cannot race this enqueue or appear in reverse order.
            drop(permit.send(JournalEntry::Checkpoint {
                change: RestartCheckpoint::Suppress { generation, turn },
                reply,
            }));
            generation
        };
        let changed = receive.await.map_err(JournalError::Reply)??;
        Ok(changed.then_some(generation))
    }
}
