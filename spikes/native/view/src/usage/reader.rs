//! Correlated, coalesced reads shared by usage totals and account limits.
use std::sync::Arc;

use crate::devices::{Ticket, Tickets};
use vessel_proto::ErrorCode;

/// Safe client failures cannot carry vendor or credential error text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RequestFailure {
    /// A connection is unavailable.
    #[error("the machine is offline")]
    Offline,
    /// Admission is exhausted; a person can retry later.
    #[error("a usage request is already in progress")]
    Busy,
    /// The peer does not implement this existing method.
    #[error("usage is unavailable on this version")]
    Unsupported,
    /// A node refusal retained as a code only.
    #[error("usage request refused ({0:?})")]
    Refused(ErrorCode),
    /// A request exceeded its fixed deadline.
    #[error("the usage request timed out")]
    TimedOut,
    /// The connection returned the wrong reply type.
    #[error("the usage reply was invalid")]
    InvalidReply,
    /// Correlation must never wrap and admit a stale response.
    #[error("usage request identities exhausted")]
    Exhausted,
}

impl RequestFailure {
    /// Safe translated action text replaces untrusted node error messages.
    #[must_use]
    pub fn message(self, words: &'static crate::localization::Translator) -> &'static str {
        let text = words.usage();
        match self {
            Self::Offline => text.offline(),
            Self::Busy => text.busy(),
            Self::Unsupported => text.unsupported(),
            Self::TimedOut => text.timeout(),
            Self::InvalidReply | Self::Exhausted => text.failed(),
            Self::Refused(code) => match code {
                ErrorCode::Forbidden | ErrorCode::Unauthorized => text.forbidden(),
                ErrorCode::ProtocolMismatch | ErrorCode::Unsupported => text.unsupported(),
                ErrorCode::Unavailable => text.busy(),
                ErrorCode::InvalidRequest
                | ErrorCode::NotFound
                | ErrorCode::Conflict
                | ErrorCode::HarnessNotInstalled
                | ErrorCode::SpawnFailed
                | ErrorCode::Internal => text.failed(),
            },
        }
    }
}

/// Refresh and failure never make retained observations look freshly fetched.
#[derive(Debug, Default)]
pub enum Phase<T> {
    /// Nothing is selected or requested.
    #[default]
    Idle,
    /// Current selection is pending; only same-selection values may remain.
    Loading {
        /// Last successful observation, with its original timestamp.
        retained: Option<Arc<T>>,
    },
    /// A successful observation of the selected scope.
    Ready(Arc<T>),
    /// Last attempt failed; retained data is explicitly not the new result.
    Failed {
        /// Sanitized reason for the failed attempt.
        reason: RequestFailure,
        /// Same-selection observation, if one was obtained before.
        retained: Option<Arc<T>>,
    },
}
impl<T> Phase<T> {
    /// Data and request status are independent; callers must render both.
    #[must_use]
    pub fn observation(&self) -> Option<&T> {
        match self {
            Self::Idle => None,
            Self::Loading { retained } | Self::Failed { retained, .. } => retained.as_deref(),
            Self::Ready(value) => Some(value),
        }
    }
    /// Agent-query errors hide values; an in-flight reread may keep a complete prior snapshot.
    /// Host queries keep data first through `observation`; account-attempt retention is separate.
    #[must_use]
    pub fn agent_observation(&self) -> Option<&T> {
        match self {
            Self::Idle | Self::Failed { .. } => None,
            Self::Loading { retained } => retained.as_deref(),
            Self::Ready(value) => Some(value),
        }
    }
    fn retained(&self) -> Option<Arc<T>> {
        match self {
            Self::Idle => None,
            Self::Loading { retained } | Self::Failed { retained, .. } => retained.clone(),
            Self::Ready(value) => Some(Arc::clone(value)),
        }
    }
}

/// One admission, carried unchanged across the link boundary.
#[derive(Debug, Clone)]
pub struct ReadRequest<K> {
    /// Correlation identity reserved before enqueueing.
    pub ticket: Ticket,
    /// Typed machine/account/range selection.
    pub selection: K,
}

/// At most one active request plus the newest choice awaiting admission.
#[derive(Debug)]
pub struct Reader<K, T> {
    selected: Option<K>,
    active: Option<Ticket>,
    queued: Option<K>,
    tickets: Tickets,
    phase: Phase<T>,
    current: Option<Ticket>,
    observed: Option<Ticket>,
}
impl<K, T> Default for Reader<K, T> {
    fn default() -> Self {
        Self {
            selected: None,
            active: None,
            queued: None,
            tickets: Tickets::default(),
            phase: Phase::Idle,
            current: None,
            observed: None,
        }
    }
}
impl<K: Clone + Eq, T> Reader<K, T> {
    /// Request state is read-only outside its lifetime owner.
    #[must_use]
    pub const fn phase(&self) -> &Phase<T> {
        &self.phase
    }
    /// Current scope, so passive frame reconciliation does not refresh forever.
    #[must_use]
    pub const fn selection(&self) -> Option<&K> {
        self.selected.as_ref()
    }
    /// Identity of the immutable observation currently drawn, including while a read is pending.
    /// Render callbacks use it to refuse edits from an older report or closed/reopened surface.
    #[must_use]
    pub const fn observation_ticket(&self) -> Option<Ticket> {
        self.observed
    }
    /// Select or explicitly refresh; intermediate pending choices are coalesced.
    #[must_use]
    pub fn request(&mut self, selection: K) -> Option<ReadRequest<K>> {
        let retained = (self.selected.as_ref() == Some(&selection))
            .then(|| self.phase.retained())
            .flatten();
        if retained.is_none() {
            self.observed = None;
        }
        self.selected = Some(selection.clone());
        self.queued = Some(selection);
        self.current = None;
        self.phase = Phase::Loading { retained };
        self.admit()
    }
    /// Closing discards publishing rights, including across reopening the same scope.
    pub fn close(&mut self) {
        self.selected = None;
        self.queued = None;
        self.current = None;
        self.phase = Phase::Idle;
        self.observed = None;
    }
    /// Settle only the active admission and then admit the latest queued choice.
    #[must_use]
    pub fn answer(
        &mut self,
        ticket: Ticket,
        result: Result<T, RequestFailure>,
    ) -> Option<ReadRequest<K>> {
        if self.active != Some(ticket) {
            return None;
        }
        self.active = None;
        if self.current == Some(ticket) {
            self.phase = match result {
                Ok(value) => {
                    self.observed = Some(ticket);
                    Phase::Ready(Arc::new(value))
                }
                Err(reason) => Phase::Failed {
                    reason,
                    retained: self.phase.retained(),
                },
            };
            self.current = None;
        }
        self.admit()
    }
    fn admit(&mut self) -> Option<ReadRequest<K>> {
        if self.active.is_some() {
            return None;
        }
        let selection = self.queued.take()?;
        match self.tickets.next_ticket() {
            Ok(ticket) => {
                self.active = Some(ticket);
                self.current = Some(ticket);
                Some(ReadRequest { ticket, selection })
            }
            Err(_exhausted) => {
                self.phase = Phase::Failed {
                    reason: RequestFailure::Exhausted,
                    retained: self.phase.retained(),
                };
                None
            }
        }
    }
}

#[cfg(test)]
mod tests;
