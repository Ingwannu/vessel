//! A row settles only its own current mutation; no optimistic pause/resume and
//! no automatic retry of a run or deletion with an uncertain network result.

use super::super::{
    Action,
    load::{Failure, MAX_IN_FLIGHT, Ticket},
};
use super::{Ask, Page, Request, RowKey, Snapshot};
use vessel_proto::automation::{ScheduleInfo, ScheduleRun};

/// Row-local progress/error, exposed to drawing without a second decision.
#[derive(Debug, Default)]
pub enum Mutation {
    /// No operation pending.
    #[default]
    Idle,
    /// One operation owns the spinner and disables repeat clicks.
    Running {
        /// Request identity.
        ticket: Ticket,
        /// What is in flight.
        action: Action,
    },
    /// No reply confirms this write; a fresh authoritative read must settle first.
    Unconfirmed {
        /// The operation whose reply was lost, never automatically repeated.
        action: Action,
    },
    /// A completed refusal, shown beside this row.
    Failed {
        /// Which action failed.
        action: Action,
        /// Typed reason.
        failure: Failure,
    },
}

impl Mutation {
    /// Whether another operation must wait.
    #[must_use]
    pub const fn pending(&self) -> bool {
        matches!(self, Self::Running { .. })
    }
    /// An uncertain write cannot be repeated before an authoritative refresh.
    #[must_use]
    pub const fn blocks_actions(&self) -> bool {
        matches!(self, Self::Running { .. } | Self::Unconfirmed { .. })
    }
    pub(super) fn reconnected(&mut self) {
        // A refusal described the previous connection; an unresolved write
        // remains uncertain even when that connection is gone.
        match *self {
            Self::Running { action, .. } => *self = Self::Unconfirmed { action },
            Self::Idle | Self::Failed { .. } => *self = Self::Idle,
            Self::Unconfirmed { .. } => {}
        }
    }
    pub(super) fn reconciled(&mut self) {
        if let Self::Unconfirmed { action } = *self {
            *self = Self::Failed {
                action,
                failure: Failure::Unconfirmed,
            };
        }
    }
    /// Operation in flight, so the row can explain its temporary lack of actions.
    #[must_use]
    pub const fn progress(&self) -> Option<Action> {
        match self {
            Self::Running { action, .. } => Some(*action),
            Self::Idle | Self::Failed { .. } | Self::Unconfirmed { .. } => None,
        }
    }
    /// The failure to word, if any.
    #[must_use]
    pub const fn failure(&self) -> Option<&Failure> {
        match self {
            Self::Failed { failure, .. } => Some(failure),
            Self::Unconfirmed { .. } => Some(&Failure::Unconfirmed),
            Self::Idle | Self::Running { .. } => None,
        }
    }
}

/// The authoritative reply to an operation, without client/network types.
#[derive(Debug)]
pub enum WriteValue {
    /// A paused or resumed definition.
    Schedule(Box<ScheduleInfo>),
    /// A run-now result; the following list refresh reads its updated state.
    Run(ScheduleRun),
    /// Delete acknowledged.
    Deleted,
}

impl Page {
    /// Validate against the current row before issuing one bounded operation.
    pub fn act(&mut self, key: &RowKey, action: Action) -> Option<Request> {
        let available = self.row(key)?.actions();
        if !available.contains(&action) {
            return None;
        }
        if self.pending() >= MAX_IN_FLIGHT {
            self.caches
                .get_mut(&key.machine)?
                .rows
                .get_mut(&key.schedule)?
                .mutation = Mutation::Failed {
                action,
                failure: Failure::Busy,
            };
            return None;
        }
        let ask = match action {
            Action::Pause => Ask::Pause(key.schedule),
            Action::Resume => Ask::Resume(key.schedule),
            Action::RunNow => Ask::Run(key.schedule),
            Action::Delete => Ask::Delete(key.schedule),
            Action::Edit => return None,
        };
        let context = self.caches.get(&key.machine)?.connection.clone()?;
        let ticket = self.issue()?;
        self.caches
            .get_mut(&key.machine)?
            .rows
            .get_mut(&key.schedule)?
            .mutation = Mutation::Running { ticket, action };
        Some(self.track(Request::on(context, ticket, ask)))
    }

    /// Apply only this operation's answer. Invalidate earlier list reads so
    /// they cannot put a deleted/paused row back as if the operation never ran.
    pub fn written(&mut self, request: &Request, answer: Result<WriteValue, Failure>) {
        let id = match request.ask {
            Ask::List => return,
            Ask::Pause(id) | Ask::Resume(id) | Ask::Run(id) | Ask::Delete(id) => id,
        };
        if !self.retire(request) {
            return;
        }
        let Some(cache) = self.caches.get_mut(&request.machine) else {
            return;
        };
        if cache.connection.as_ref() != Some(request.connection()) {
            return;
        }
        let Some(row) = cache.rows.get_mut(&id) else {
            return;
        };
        let Mutation::Running { ticket, action } = row.mutation else {
            return;
        };
        if ticket != request.ticket {
            return;
        }
        let answer = answer
            .and_then(|value| value.checked(request.ask, id))
            .map_err(|failure| match failure {
                Failure::Unexpected => Failure::Unconfirmed,
                Failure::Node(_)
                | Failure::Older
                | Failure::NotEnabled
                | Failure::Offline
                | Failure::ConnectionChanged
                | Failure::TimedOut
                | Failure::Busy
                | Failure::Unconfirmed
                | Failure::Exhausted
                | Failure::Preferences
                | Failure::NoTransport => failure,
            });
        match answer {
            Ok(value) => {
                row.mutation = Mutation::Idle;
                if let super::read::Read::Ready { snapshot, .. } = &mut cache.read {
                    value.apply(id, snapshot);
                }
                cache.read.invalidate();
            }
            Err(failure) => {
                if let Failure::Node(error) = &failure {
                    row.authority.refused(error.code);
                }
                if failure == Failure::Unconfirmed {
                    row.mutation = Mutation::Unconfirmed { action };
                    cache.read.invalidate();
                } else {
                    row.mutation = Mutation::Failed { action, failure };
                }
            }
        }
    }
}

impl WriteValue {
    // An unknown shape must never be accepted merely because it arrived for
    // the current ticket. All operations and payload variants stay exhaustive.
    fn checked(self, ask: Ask, id: vessel_proto::ScheduleId) -> Result<Self, Failure> {
        let valid = match (&self, ask) {
            (Self::Schedule(info), Ask::Pause(_) | Ask::Resume(_)) => info.id == id,
            (Self::Run(_), Ask::Run(_)) | (Self::Deleted, Ask::Delete(_)) => true,
            (Self::Schedule(_), Ask::List | Ask::Run(_) | Ask::Delete(_))
            | (Self::Run(_), Ask::List | Ask::Pause(_) | Ask::Resume(_) | Ask::Delete(_))
            | (Self::Deleted, Ask::List | Ask::Pause(_) | Ask::Resume(_) | Ask::Run(_)) => false,
        };
        if valid {
            Ok(self)
        } else {
            Err(Failure::Unconfirmed)
        }
    }

    fn apply(self, id: vessel_proto::ScheduleId, snapshot: &mut Snapshot) {
        match self {
            Self::Schedule(info) => {
                if let Some(old) = snapshot.schedules.iter_mut().find(|old| old.id == id) {
                    *old = *info;
                }
            }
            Self::Run(run) => {
                if let Some(old) = snapshot.schedules.iter_mut().find(|old| old.id == id) {
                    old.last_run = Some(run);
                }
            }
            Self::Deleted => snapshot.schedules.retain(|old| old.id != id),
        }
    }
}
