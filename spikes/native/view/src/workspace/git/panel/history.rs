//! Lazy history projection after Paseo use-commits-query.ts. A retained useful
//! answer may stay drawn while inactive; old nodes never invent classification.

use super::Demand;
use crate::workspace::problem::Problem;
use std::time::Duration;
use vessel_proto::workspace::{CommitInfo, RefName};

/// Re-expanding a history section may use a recent cached answer.
pub const HISTORY_FRESH_FOR: Duration = Duration::from_secs(30);
/// Immutable file bodies can be retained independently of branch changes.
pub const COMMIT_FILE_RETAIN_FOR: Duration = Duration::from_secs(5 * 60);

/// Both history listing and base classification must be implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryCapability {
    /// An admitted node method supplies the complete contract.
    Supported,
    /// A legacy log is insufficient to classify branch/base commits.
    Unsupported,
}

/// Base history supplies graph continuity but is not shown as branch commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitLocation {
    /// A commit ahead of the comparison base.
    Branch,
    /// A commit on the base, retained for graph ancestry only.
    Base,
}

/// Unknown publication must not be painted as definitely unpushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitPublication {
    /// Present in the authoritative remote-tracking history.
    Published,
    /// Known absent from that remote-tracking history.
    Local,
    /// No usable remote-tracking comparison exists.
    Unknown,
}

/// Preserve the entire typed node commit rather than just a display subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCommit {
    /// Original id, parents, author, time and subject.
    pub commit: CommitInfo,
    /// Classification supplied by the node.
    pub location: CommitLocation,
    /// Remote publication supplied independently of base classification.
    pub publication: CommitPublication,
}

/// Query data, retained independently of section visibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryData {
    /// Selected base, if the node could resolve one.
    pub base: Option<RefName>,
    /// Branch plus bounded base history in the node's graph order.
    pub commits: Vec<HistoryCommit>,
}

impl HistoryData {
    /// Only branch commits form visible section rows, as the reference does.
    pub fn branch_commits(&self) -> impl Iterator<Item = &HistoryCommit> {
        self.commits
            .iter()
            .filter(|commit| commit.location == CommitLocation::Branch)
    }
}

/// Placeholder data belongs to an earlier query key and may not be displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryCache<'a> {
    /// No result, or a placeholder for another repository/base.
    Missing,
    /// Data belongs to the current repository/base identity.
    Current(&'a HistoryData),
}

/// Every reference query state is explicit before the screen draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryState<'a> {
    /// This node cannot provide classified history.
    Unsupported,
    /// Hidden/collapsed, without useful cached data.
    Idle,
    /// Expanded but disconnected, without useful cached data.
    Connecting,
    /// Expanded, connected, awaiting its first answer.
    Loading,
    /// Failed without a usable answer. Retry remains available.
    Failed(&'a Problem),
    /// Current data remains useful even while the section is inactive.
    Loaded(&'a HistoryData),
}

/// Match the reference's capability/data/demand/connection/error precedence.
#[must_use]
pub fn history_state<'a>(
    capability: HistoryCapability,
    demand: Demand,
    cache: HistoryCache<'a>,
    failure: Option<&'a Problem>,
) -> HistoryState<'a> {
    match capability {
        HistoryCapability::Unsupported => return HistoryState::Unsupported,
        HistoryCapability::Supported => {}
    }
    match cache {
        HistoryCache::Current(data) => return HistoryState::Loaded(data),
        HistoryCache::Missing => {}
    }
    match demand {
        Demand::Inactive => HistoryState::Idle,
        Demand::Disconnected => HistoryState::Connecting,
        Demand::Active => failure.map_or(HistoryState::Loading, HistoryState::Failed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_query_states_have_capability_data_demand_and_error_precedence() {
        let data = HistoryData {
            base: None,
            commits: Vec::new(),
        };
        let failure = Problem::Unsent;
        for demand in [Demand::Inactive, Demand::Disconnected, Demand::Active] {
            assert_eq!(
                history_state(
                    HistoryCapability::Unsupported,
                    demand,
                    HistoryCache::Current(&data),
                    Some(&failure)
                ),
                HistoryState::Unsupported
            );
            assert_eq!(
                history_state(
                    HistoryCapability::Supported,
                    demand,
                    HistoryCache::Current(&data),
                    Some(&failure)
                ),
                HistoryState::Loaded(&data)
            );
        }
        for (demand, failure, expected) in [
            (Demand::Inactive, Some(&failure), HistoryState::Idle),
            (
                Demand::Disconnected,
                Some(&failure),
                HistoryState::Connecting,
            ),
            (
                Demand::Active,
                Some(&failure),
                HistoryState::Failed(&failure),
            ),
            (Demand::Active, None, HistoryState::Loading),
        ] {
            assert_eq!(
                history_state(
                    HistoryCapability::Supported,
                    demand,
                    HistoryCache::Missing,
                    failure
                ),
                expected
            );
        }
    }
}
