//! Classified branch history for the pending git panel methods (ADR0087).
//! File summaries/bodies remain lazy immutable-commit comparison reads.
use super::{CommitAbbreviation, CommitInfo, RefName, WorkspaceRoot};
use serde::{Deserialize, Serialize};

/// Pending branch history, authorized and re-resolved by workspace root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadGitHistory {
    /// The actual registered repository, never a client-supplied directory.
    pub root: WorkspaceRoot,
}

/// Whether the commit belongs to this branch's work or base context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum GitCommitLocation {
    /// Reachable from HEAD and not the chosen base.
    Workspace,
    /// Up to ten ancestors beginning at the actual fork point.
    Base,
    /// A classification this client does not know.
    #[serde(other)]
    Unknown,
}

/// Publication is checked against every observed remote ref, not just upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum GitCommitPublication {
    /// Not reachable from any remote ref the repository has observed.
    Local,
    /// Reachable from at least one observed remote ref; no fetch is performed.
    Published,
    /// A classification this client does not know.
    #[serde(other)]
    Unknown,
}

/// Complete metadata, classified without discarding base-context commits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GitHistoryCommit {
    /// ID, all parents, author, author time and complete subject.
    pub commit: CommitInfo,
    /// Git’s observed unique abbreviation; requests still use the full commit ID.
    pub short: CommitAbbreviation,
    /// Base rows are retained even when the native section hides them.
    pub location: GitCommitLocation,
    /// A remote membership fact, never guessed from the branch's ahead count.
    pub publication: GitCommitPublication,
}

/// All workspace commits and the bounded base context, newest in each group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GitHistory {
    /// The exact selected local/remote stream, absent while on the base branch.
    pub base: Option<RefName>,
    /// Workspace commits first, followed by up to ten fork-point ancestors.
    pub commits: Vec<GitHistoryCommit>,
}

/// A complete history and a bounded/base refusal must not share an empty list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GitHistoryRead {
    /// An explicitly qualified recorded base no longer resolves.
    BaseUnavailable {
        /// Preserve the exact requested stream for the person's next step.
        base: RefName,
    },
    /// The protected read exceeded its output budget; no partial history is shown.
    TooLarge,
    /// Complete branch metadata and bounded base context.
    Ready {
        /// The successful read, including a genuinely empty detached/unborn result.
        data: GitHistory,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Method, MethodShape, Request as _, Scope};
    #[test]
    fn history_remains_pending_read_unary_and_does_not_change_legacy_log() {
        assert_eq!(ReadGitHistory::METHOD, Method::GitHistory);
        assert_eq!(Method::GitHistory.scope(), Some(Scope::Read));
        assert_eq!(Method::GitHistory.shape(), MethodShape::Unary);
        assert_eq!(Method::GitHistory.since(), None);
        assert_eq!(
            Method::GitLog.since(),
            Some(crate::ProtocolVersion::new(10))
        );
    }
    #[test]
    fn empty_ready_missing_base_and_output_limit_keep_different_whole_results() {
        let values = [
            GitHistoryRead::Ready {
                data: GitHistory {
                    base: None,
                    commits: Vec::new(),
                },
            },
            GitHistoryRead::BaseUnavailable {
                base: "refs/heads/renamed".parse().unwrap(),
            },
            GitHistoryRead::TooLarge,
        ];
        let encoded = values
            .iter()
            .map(|value| serde_json::to_string(value).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            encoded,
            [
                r#"{"kind":"ready","data":{"base":null,"commits":[]}}"#,
                r#"{"kind":"base_unavailable","base":"refs/heads/renamed"}"#,
                r#"{"kind":"too_large"}"#
            ]
        );
        for (value, text) in values.iter().zip(encoded) {
            assert_eq!(
                serde_json::from_str::<GitHistoryRead>(&text).unwrap(),
                *value
            );
        }
        assert_eq!(
            serde_json::from_str::<GitCommitLocation>(r#""future""#).unwrap(),
            GitCommitLocation::Unknown
        );
        assert_eq!(
            serde_json::from_str::<GitCommitPublication>(r#""future""#).unwrap(),
            GitCommitPublication::Unknown
        );
    }
}
