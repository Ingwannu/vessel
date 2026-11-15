//! Initial reads and background refreshes are distinct. A failed refresh keeps
//! the last node-authored rows with an explicit failure, never a false empty list.

use super::super::load::{Failure, Ticket};
use chrono::{DateTime, Utc};
use vessel_proto::automation::ScheduleInfo;
use vessel_proto::workspace::{ProjectInfo, WorktreeInfo};

const REFRESH_AFTER: chrono::TimeDelta = chrono::TimeDelta::seconds(5);

/// One authoritative list read plus target facts from the same machine.
/// Display clocks belong to the presentation cache, separate from node state.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Node-authored schedules, in the node's bounded list.
    pub schedules: Vec<ScheduleInfo>,
    /// Client read time, for refresh cadence only, never schedule state.
    pub read_at: DateTime<Utc>,
    /// Worktree details only when a schedule targets one; a failed lookup
    /// remains distinct from an empty list and never means the target is gone.
    pub worktrees: Result<Vec<WorktreeInfo>, Failure>,
    /// Node project facts read for target names; no dependency on visiting another screen.
    pub projects: Result<Vec<ProjectInfo>, Failure>,
}

#[derive(Debug, Default)]
pub(super) enum Refresh {
    #[default]
    Idle,
    Needed,
    Waiting(Ticket),
    Failed(Failure),
}

#[derive(Debug, Default)]
pub(super) enum Read {
    #[default]
    Unread,
    Loading(Ticket),
    Ready {
        snapshot: Snapshot,
        refresh: Refresh,
    },
    Failed(Failure),
}

impl Read {
    pub(super) fn pending(&self) -> Option<Ticket> {
        match self {
            Self::Loading(ticket)
            | Self::Ready {
                refresh: Refresh::Waiting(ticket),
                ..
            } => Some(*ticket),
            Self::Unread
            | Self::Ready {
                refresh: Refresh::Idle | Refresh::Needed | Refresh::Failed(_),
                ..
            }
            | Self::Failed(_) => None,
        }
    }

    pub(super) fn due(&self, now: DateTime<Utc>) -> bool {
        match self {
            Self::Unread
            | Self::Ready {
                refresh: Refresh::Needed,
                ..
            } => true,
            Self::Ready {
                snapshot,
                refresh: Refresh::Idle,
            } => now.signed_duration_since(snapshot.read_at) >= REFRESH_AFTER,
            Self::Loading(_)
            | Self::Ready {
                refresh: Refresh::Waiting(_) | Refresh::Failed(_),
                ..
            }
            | Self::Failed(_) => false,
        }
    }

    pub(super) fn begin(&mut self, ticket: Ticket) {
        match self {
            Self::Ready { refresh, .. } => *refresh = Refresh::Waiting(ticket),
            Self::Unread | Self::Loading(_) | Self::Failed(_) => *self = Self::Loading(ticket),
        }
    }

    pub(super) fn invalidate(&mut self) {
        match self {
            Self::Ready { refresh, .. } => *refresh = Refresh::Needed,
            Self::Unread | Self::Loading(_) | Self::Failed(_) => *self = Self::Unread,
        }
    }

    pub(super) fn settle(&mut self, ticket: Ticket, answer: Result<Snapshot, Failure>) -> bool {
        if self.pending() != Some(ticket) {
            return false;
        }
        match answer {
            Ok(snapshot) => {
                *self = Self::Ready {
                    snapshot,
                    refresh: Refresh::Idle,
                }
            }
            Err(error) => self.fail(error),
        }
        true
    }

    pub(super) fn fail(&mut self, failure: Failure) {
        match self {
            Self::Ready { refresh, .. } => *refresh = Refresh::Failed(failure),
            Self::Unread | Self::Loading(_) | Self::Failed(_) => *self = Self::Failed(failure),
        }
    }

    pub(super) const fn snapshot(&self) -> Option<&Snapshot> {
        match self {
            Self::Ready { snapshot, .. } => Some(snapshot),
            Self::Unread | Self::Loading(_) | Self::Failed(_) => None,
        }
    }

    pub(super) const fn problem(&self) -> Option<&Failure> {
        match self {
            Self::Failed(error)
            | Self::Ready {
                refresh: Refresh::Failed(error),
                ..
            } => Some(error),
            Self::Unread
            | Self::Loading(_)
            | Self::Ready {
                refresh: Refresh::Idle | Refresh::Needed | Refresh::Waiting(_),
                ..
            } => None,
        }
    }
}
