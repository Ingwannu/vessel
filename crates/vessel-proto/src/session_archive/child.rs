//! Finished-child archive shapes; no Request implementation until current parentage is wired.
use crate::{Activity, SessionInfo, SessionRef, SessionStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Route to the child's node and compare the complete current parent locally.
/// This describes a staged operation, not an assigned/registered RPC yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ArchiveChildSession {
    /// Exact expected attachment, including a parent on another node.
    pub parent: SessionRef,
    /// Exact child; another node with the same session UUID is a different target.
    pub child: SessionRef,
}

/// A changed snapshot is skipped, rather than reported as a successful archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ChildArchiveSkip {
    /// The child detached or acquired a different current parent.
    ParentChanged,
    /// Its durable archive mark already exists.
    AlreadyArchived,
    /// It is active, waiting for input, starting or no longer known finished.
    NotFinished,
    /// A future refusal never gains archive authority in an older consumer.
    #[serde(other)]
    Unknown,
}

/// A per-child outcome preserves the routed identity and node-owned timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
#[must_use]
pub enum ChildArchiveOutcome {
    /// Current parent and final idle admission agreed; the durable mark committed.
    Archived {
        /// Exact child that ended and was archived.
        child: SessionRef,
        /// Timestamp from its committed archive mark.
        archived_at: DateTime<Utc>,
    },
    /// No archive was issued for this ineligible child.
    Skipped {
        /// Exact child considered.
        child: SessionRef,
        /// Why its current state no longer qualifies.
        reason: ChildArchiveSkip,
    },
    /// Unknown outcomes must not be counted as archived or silently retried.
    #[serde(other)]
    Unknown,
}

/// Snapshot eligibility for the managed idle/error rows in archive-finished.
/// This is not mutation authority: current parent must also match under the node
/// admission lock, followed by actor IfIdle admission on a live structured child.
/// Missing board facts never imply idle, and display/PR phrases are not activity.
#[must_use]
pub fn finished_child_candidate(info: &SessionInfo) -> bool {
    if info.marks.archived_at.is_some() {
        return false;
    }
    match info.status {
        SessionStatus::Running => info
            .board
            .as_ref()
            .is_some_and(|board| board.activity == Activity::Idle),
        SessionStatus::Failed { .. } => true,
        SessionStatus::Starting | SessionStatus::Exited { .. } | SessionStatus::Interrupted => {
            false
        }
    }
}

#[cfg(test)]
mod tests;
