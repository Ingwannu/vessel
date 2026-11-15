//! Metadata navigation owns paging and correlation, never transcript bodies or I/O.
//! A full Follow remains the compatibility fallback for peers without this call.

use crate::connection::{ConnectionContext, MachineKey};
use vessel_proto::prompt_index::{
    INDEX_ENTRIES, ListSessionPrompts, PromptIndexEpoch, PromptIndexLimit, PromptIndexPage,
    PromptLocation,
};
use vessel_proto::{ErrorCode, EventSeq, NodeId, SessionId};

mod rows;
pub use rows::{PromptRow, PromptRows};

/// Session identity includes its node-book destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexTarget {
    /// Already admitted node-book key.
    pub machine: MachineKey,
    /// The shared immutable admission captured from the machine snapshot.
    pub connection: Option<ConnectionContext>,
    /// The retained log to navigate.
    pub session: SessionId,
}

/// Identity is never reused when switching sessions or restarting a page set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexTicket(u64);

/// Safe failure projection; no node-supplied text enters a notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IndexProblem {
    /// No connection or a transport failure.
    #[error("the prompt-index connection is unavailable")]
    Connection,
    /// Pending or too-old protocol, including the browser demo surface.
    #[error("the peer does not support prompt metadata")]
    Unsupported,
    /// Retention or rewind invalidated this set of addresses.
    #[error("the prompt history changed")]
    HistoryChanged,
    /// A page violates session, epoch, ordering or total-size invariants.
    #[error("the prompt metadata reply is inconsistent")]
    Invalid,
    /// Work admission or an app command queue is full.
    #[error("prompt metadata is busy")]
    Busy,
    /// Bounded work did not finish by its deadline.
    #[error("the prompt metadata deadline elapsed")]
    Timeout,
    /// A node explicitly refused, with a safe structured code.
    #[error("the prompt metadata request was refused ({0:?})")]
    Refused(ErrorCode),
    /// No unused correlation identity remains.
    #[error("prompt request identities are exhausted")]
    Exhausted,
}

/// The index's public lifecycle; partial page sets never become navigable.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum IndexPhase {
    /// No session is selected or outline is disabled.
    #[default]
    Hidden,
    /// Exactly one page is outstanding for this set.
    Reading(IndexTicket),
    /// Every page belongs to one consistent history revision.
    Ready,
    /// Use the complete followed log until another request succeeds.
    Failed(IndexProblem),
}

/// A typed boundary request, also used by the in-memory adapter tests.
#[derive(Debug, Clone)]
pub struct IndexAsk {
    /// Correlation identity of the complete paged set.
    pub ticket: IndexTicket,
    /// Node and session the answer must belong to.
    pub target: IndexTarget,
    /// The actual node request; no parallel wire shape.
    pub request: ListSessionPrompts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Source {
    target: IndexTarget,
    latest: Option<EventSeq>,
}

/// One paged index plus an app-lifetime identity source.
#[derive(Debug, Default)]
pub struct PromptIndex {
    source: Option<Source>,
    phase: IndexPhase,
    serial: u64,
    epoch: Option<PromptIndexEpoch>,
    after: Option<EventSeq>,
    entries: Vec<PromptLocation>,
    dirty: bool,
    rewound: Option<EventSeq>,
    /// History guard only; never authorizes a call without the shared admission.
    history_node: Option<NodeId>,
}

impl PromptIndex {
    /// Drop addresses without resetting identities (A→B→A must reject A's old reply).
    pub fn clear(&mut self) {
        self.source = None;
        self.phase = IndexPhase::Hidden;
        self.epoch = None;
        self.after = None;
        self.entries.clear();
        self.dirty = false;
        self.rewound = None;
        self.history_node = None;
    }

    /// Retire addresses/work while preserving the known history and identity source.
    pub fn unavailable(&mut self) {
        self.phase = if self.source.is_some() {
            IndexPhase::Failed(IndexProblem::Connection)
        } else {
            IndexPhase::Hidden
        };
        self.epoch = None;
        self.after = None;
        self.entries.clear();
        self.dirty = false;
    }

    /// The newest loaded prompt changed. Coalesce streaming updates into one refresh.
    #[must_use]
    pub fn refresh(&mut self, target: IndexTarget, latest: Option<EventSeq>) -> Option<IndexAsk> {
        let source = Source { target, latest };
        if self.source.as_ref() == Some(&source) {
            return None;
        }
        let same = self
            .source
            .as_ref()
            .is_some_and(|old| old.target == source.target);
        let same_address = self.source.as_ref().is_some_and(|old| {
            old.target.machine == source.target.machine
                && old.target.session == source.target.session
        });
        let node = source
            .target
            .connection
            .as_ref()
            .map(ConnectionContext::node);
        // A missing fleet admission is not proof that the durable history changed.
        // Keep the last observed node only for this guard, never for call selection.
        let same_history = same_address
            && (node.is_none() || self.history_node.is_none() || node == self.history_node);
        if !same_history {
            self.rewound = None;
        }
        if !same_address || node.is_some() {
            self.history_node = node;
        }
        self.source = Some(source);
        if same {
            match self.phase {
                IndexPhase::Reading(_) => {
                    self.dirty = true;
                    return None;
                }
                IndexPhase::Failed(IndexProblem::Unsupported) => return None,
                IndexPhase::Hidden | IndexPhase::Ready | IndexPhase::Failed(_) => {}
            }
        }
        self.start()
    }

    /// Explicit retry, reopen or rewind discards the prior revision before asking.
    #[must_use]
    pub fn restart(&mut self) -> Option<IndexAsk> {
        self.start()
    }

    /// A rewind observed on Follow invalidates replies computed before it.
    #[must_use]
    pub fn history_changed(&mut self, seq: EventSeq) -> Option<IndexAsk> {
        if self.rewound == Some(seq) {
            return None;
        }
        self.rewound = Some(seq);
        self.start()
    }

    fn start(&mut self) -> Option<IndexAsk> {
        let _present = self.source.as_ref()?;
        let Some(serial) = self.serial.checked_add(1) else {
            self.phase = IndexPhase::Failed(IndexProblem::Exhausted);
            return None;
        };
        self.serial = serial;
        self.phase = IndexPhase::Reading(IndexTicket(serial));
        self.epoch = None;
        self.after = None;
        self.entries.clear();
        self.dirty = false;
        self.ask()
    }

    fn ask(&self) -> Option<IndexAsk> {
        let source = self.source.as_ref()?;
        let IndexPhase::Reading(ticket) = self.phase else {
            return None;
        };
        Some(IndexAsk {
            ticket,
            target: source.target.clone(),
            request: ListSessionPrompts {
                session: source.target.session,
                after: self.after,
                epoch: self.epoch,
                limit: PromptIndexLimit::default(),
            },
        })
    }

    /// Accept only the current session/ticket and strictly advancing pages of one epoch.
    #[must_use]
    pub fn answered(
        &mut self,
        ask: &IndexAsk,
        result: Result<PromptIndexPage, IndexProblem>,
    ) -> Option<IndexAsk> {
        if self.phase != IndexPhase::Reading(ask.ticket)
            || self
                .source
                .as_ref()
                .is_none_or(|source| source.target != ask.target)
            || self.after != ask.request.after
        {
            return None;
        }
        let page = match result {
            Ok(page) => page,
            Err(problem) => {
                self.phase = IndexPhase::Failed(problem);
                return None;
            }
        };
        let failure = if self.epoch.is_some_and(|epoch| epoch != page.epoch())
            || self.rewound.is_some_and(|seq| {
                // Once retention passes the observed marker, it is absent from
                // the retained log. That is not an older pre-rewind snapshot.
                page.epoch().retained_from() <= seq && page.epoch().rewound_at() != Some(seq)
            }) {
            Some(IndexProblem::HistoryChanged)
        } else if page.session() != ask.target.session
            || page
                .entries()
                .first()
                .is_some_and(|entry| self.after.is_some_and(|after| entry.seq <= after))
            || self.entries.len().saturating_add(page.entries().len()) > INDEX_ENTRIES
        {
            Some(IndexProblem::Invalid)
        } else {
            None
        };
        if let Some(problem) = failure {
            self.phase = IndexPhase::Failed(problem);
            return None;
        }
        self.epoch = Some(page.epoch());
        self.after = page.next();
        self.entries.extend(page.entries().iter().cloned());
        if self.after.is_some() {
            return self.ask();
        }
        if self.dirty {
            return self.start();
        }
        self.phase = IndexPhase::Ready;
        None
    }

    /// Consistent metadata only; a failed or incomplete set is never exposed.
    #[must_use]
    pub fn entries(&self) -> Option<&[PromptLocation]> {
        (self.phase == IndexPhase::Ready).then_some(self.entries.as_slice())
    }

    /// The exact welcome/session scope, including while pages are in flight.
    #[must_use]
    pub fn target(&self) -> Option<&IndexTarget> {
        self.source.as_ref().map(|source| &source.target)
    }

    /// The revision accepted for these addresses, when known.
    #[must_use]
    pub const fn epoch(&self) -> Option<PromptIndexEpoch> {
        self.epoch
    }

    /// Public phase for adapters and deterministic fixture checks.
    #[must_use]
    pub const fn phase(&self) -> IndexPhase {
        self.phase
    }
}

#[cfg(test)]
mod tests;
