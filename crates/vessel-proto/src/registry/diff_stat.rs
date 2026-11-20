//! The diff stat `workspace.list` and `workspace.get` serve (ADR 0120).
//!
//! Not a registry field: a file save must not bump the registry revision.

use serde::{Deserialize, Serialize};

use super::workspace::{DiffStatRevision, WorkspaceDescriptor};
use crate::ProtocolVersion;

/// `workspace.list` / `workspace.get` diff stat (ADR 0120).
///
/// Drafted 37. `None` until the integrator assigns the number at merge;
/// until then the node does not serve the fields.
pub const DIFF_STAT_PROTOCOL: Option<ProtocolVersion> = None;

/// Whether `protocol` is served the diff stat.
#[must_use]
pub fn serves_diff_stat(protocol: ProtocolVersion) -> bool {
    DIFF_STAT_PROTOCOL.is_some_and(|since| protocol >= since)
}

/// Additions and deletions for one checkout.
///
/// The node sends this only when at least one side is non-zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DiffStat {
    /// Lines added.
    pub additions: u32,
    /// Lines removed.
    pub deletions: u32,
}

/// A workspace as list and get serve it.
///
/// The descriptor's fields stay at the top level, so a client that only
/// knows the descriptor still decodes the row and ignores the two added
/// fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ServedWorkspace {
    /// The registry row.
    #[serde(flatten)]
    pub workspace: WorkspaceDescriptor,
    /// The checkout's line counts. Absent when there is nothing to show.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_stat: Option<DiffStat>,
    /// The generation shared by every row of this answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stat_revision: Option<DiffStatRevision>,
}

impl ServedWorkspace {
    /// The descriptor with no diff stat, which is what every peer sees
    /// until the protocol is assigned.
    #[must_use]
    pub fn bare(workspace: WorkspaceDescriptor) -> Self {
        Self {
            workspace,
            diff_stat: None,
            stat_revision: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DIFF_STAT_PROTOCOL, DiffStat, serves_diff_stat};
    use crate::ProtocolVersion;
    use crate::registry::workspace::{DiffStatRevision, ListWorkspaces};

    #[test]
    fn the_diff_stat_stays_off_the_wire_until_assigned() {
        assert!(DIFF_STAT_PROTOCOL.is_none());
        assert!(!serves_diff_stat(ProtocolVersion::new(36)));
    }

    #[test]
    fn diff_stat_json_names_its_counts() {
        let stat = DiffStat {
            additions: 2,
            deletions: 1,
        };
        let json = serde_json::to_string(&stat).unwrap();
        assert_eq!(json, "{\"additions\":2,\"deletions\":1}");
    }

    #[test]
    fn a_revision_is_a_number_and_an_absent_cursor_stays_absent() {
        assert_eq!(
            serde_json::to_string(&DiffStatRevision::of(3)).unwrap(),
            "3"
        );
        let value = serde_json::to_value(ListWorkspaces::default()).unwrap();
        assert!(value.get("changed_from").is_none());
        let parsed: ListWorkspaces = serde_json::from_value(value).unwrap();
        assert!(parsed.changed_from.is_none());
    }
}
