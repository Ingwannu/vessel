//! ADR 0087 contract: methods register pending in the single Quality B table.
//! Legacy `DiffScope` stays unchanged; only the integrator assigns admission.
//!
//! [Decision Log]
//! - Purpose: committed-only must exclude the index and worktree, unlike Base.
//! - Constraint: adding a legacy scope variant could reach an old node through
//!   an already-admitted call. Existing calls must keep their exact meaning.
//! - Choice: a separate typed comparison request, staged until admission.
//! - Effect: new clients can refuse unsupported comparison explicitly; no
//!   patch parsing, command arguments or absolute paths cross this boundary.

use serde::{Deserialize, Serialize};

use super::{CommitId, RefName, RelPath, WorkspaceRoot, WorktreeBase};

/// Pending until the integrator assigns the completed git panel package.
/// This registers methods through Quality B without inventing a version.
pub const GIT_PANEL: Option<crate::ProtocolVersion> = None;

/// A successful read distinguishes absent comparison, bounded refusal and data.
/// Partial coverage stays in the existing summary's `more` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GitCompared<T> {
    /// No registered or explicit base exists for this comparison.
    NoBase,
    /// Both facts are retained without declaring either branch correct.
    BaseMismatch {
        /// The ref recorded when this worktree was created.
        stored: RefName,
        /// The different base selected by the caller.
        requested: WorktreeBase,
    },
    /// The protected reader refused a complete result past its output budget.
    TooLarge,
    /// The typed bounded summary or body, preserving all of its fields.
    Ready {
        /// Payload from the existing git parser, not text interpreted by a UI.
        data: T,
    },
}

/// Which two versions a git document compares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GitComparison {
    /// HEAD against working files, including staged and untracked changes.
    Uncommitted,
    /// Merge base against HEAD; working changes never contribute.
    Committed {
        /// Absent means the registered worktree/project base.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base: Option<WorktreeBase>,
    },
    /// Merge base against working files, including new files.
    All {
        /// Absent means the registered worktree/project base.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base: Option<WorktreeBase>,
    },
    /// One immutable commit against its first parent (empty tree for a root).
    Commit {
        /// Already parsed before any git process is started.
        id: CommitId,
    },
}

/// A typed preference in query/cache identity, not a free-form git option.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum GitWhitespace {
    /// Preserve whitespace-only changes.
    #[default]
    Show,
    /// Ignore whitespace-only changes.
    Ignore,
}

impl GitWhitespace {
    /// Adapt to the protected runner's existing switch at that boundary only.
    #[must_use]
    pub const fn ignored(self) -> bool {
        matches!(self, Self::Ignore)
    }
}

/// A forced refresh always carries a typed reason, never an unchecked flag.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum GitReadReason {
    /// Ordinary demand uses the bounded freshness cache.
    #[default]
    View,
    /// A person explicitly asked for a fresh comparison; this never fetches.
    Refresh,
    /// A known repository write requires facts after that mutation.
    Mutation,
    /// Observation recovered and needs a fresh baseline.
    Recovery,
}

impl GitReadReason {
    /// Ordinary reads omit this default on the pending wire contract.
    #[must_use]
    pub const fn is_view(&self) -> bool {
        matches!(self, Self::View)
    }
}

/// Pending `git.compare`: no arbitrary directory or command text is accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CompareGit {
    /// Re-resolved and authorized by the node, including on a cache hit.
    pub root: WorkspaceRoot,
    /// Exactly the selected comparison, never an approximate legacy scope.
    pub comparison: GitComparison,
    /// Included in the cache key so filtered output cannot poison other views.
    #[serde(default)]
    pub whitespace: GitWhitespace,
    /// Bypassing freshness requires an explicit reason.
    #[serde(default, skip_serializing_if = "GitReadReason::is_view")]
    pub reason: GitReadReason,
}

/// Pending `git.compare_file`: one bounded file body in the same comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CompareGitFile {
    /// Repository/comparison/whitespace identity retained across pages.
    pub query: CompareGit,
    /// A validated path relative to the root.
    pub path: RelPath,
    /// Hunk offset, not a byte offset or an array index in the client.
    #[serde(default)]
    pub from_hunk: GitHunkOffset,
}

/// A continuation supplied by the node; zero requests the first bounded page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct GitHunkOffset(u32);

impl GitHunkOffset {
    /// Preserve an existing node continuation without narrowing it.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Convert only at the existing file-diff runner boundary.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparisons_round_trip_without_aliasing_all_and_committed() {
        for comparison in [
            GitComparison::Uncommitted,
            GitComparison::Committed { base: None },
            GitComparison::All {
                base: Some(WorktreeBase::Ref {
                    name: "origin/main".parse().unwrap(),
                }),
            },
            GitComparison::Commit {
                id: "a".repeat(40).parse().unwrap(),
            },
        ] {
            let encoded = serde_json::to_string(&comparison).unwrap();
            assert_eq!(
                serde_json::from_str::<GitComparison>(&encoded).unwrap(),
                comparison
            );
        }
        assert_ne!(
            serde_json::to_string(&GitComparison::Committed { base: None }).unwrap(),
            serde_json::to_string(&GitComparison::All { base: None }).unwrap()
        );
    }

    #[test]
    fn comparison_file_rejects_escaping_paths_before_the_runner() {
        let root = WorkspaceRoot::Project {
            project: crate::ProjectId::random(),
        };
        let request = CompareGitFile {
            query: CompareGit {
                root,
                comparison: GitComparison::Uncommitted,
                whitespace: GitWhitespace::Show,
                reason: GitReadReason::View,
            },
            path: "src/main.rs".parse().unwrap(),
            from_hunk: GitHunkOffset::new(2),
        };
        let encoded = serde_json::to_string(&request).unwrap();
        assert_eq!(
            serde_json::from_str::<CompareGitFile>(&encoded).unwrap(),
            request
        );
        assert!(
            serde_json::from_str::<CompareGitFile>(&encoded.replace("src/main.rs", "../outside"))
                .is_err()
        );
    }
    #[test]
    fn pending_comparisons_have_read_scope_unary_shape_and_no_assigned_protocol() {
        use crate::{Method, MethodShape, Request, Scope};
        assert_eq!(CompareGit::METHOD, Method::GitCompare);
        assert_eq!(CompareGitFile::METHOD, Method::GitCompareFile);
        for method in [Method::GitCompare, Method::GitCompareFile] {
            assert_eq!(method.scope(), Some(Scope::Read));
            assert_eq!(method.shape(), MethodShape::Unary);
            assert_eq!(method.since(), None);
            // This runtime package supplies handlers; the separately reviewed
            // values-only leaf remains staged on the shared line until joined.
            assert!(!method.is_staged());
        }
    }

    #[test]
    fn no_base_limit_and_an_empty_success_have_different_wire_results() {
        use super::super::DiffSummary;
        let values = [
            GitCompared::NoBase,
            GitCompared::TooLarge,
            GitCompared::Ready {
                data: DiffSummary {
                    from: None,
                    files: vec![],
                    more: false,
                },
            },
        ];
        let encoded: Vec<_> = values
            .iter()
            .map(|value| serde_json::to_string(value).unwrap())
            .collect();
        for (value, text) in values.iter().zip(&encoded) {
            assert_eq!(
                &serde_json::from_str::<GitCompared<DiffSummary>>(text).unwrap(),
                value
            );
        }
        assert_ne!(encoded[0], encoded[1]);
        assert_ne!(encoded[0], encoded[2]);
        assert_ne!(encoded[1], encoded[2]);
    }

    #[test]
    fn base_mismatch_preserves_both_ref_identities_on_the_wire() {
        let value: GitCompared<super::super::DiffSummary> = GitCompared::BaseMismatch {
            stored: "refs/remotes/upstream/main".parse().unwrap(),
            requested: WorktreeBase::Ref {
                name: "refs/heads/main".parse().unwrap(),
            },
        };
        let encoded = serde_json::to_value(&value).unwrap();
        assert_eq!(
            encoded,
            serde_json::json!({
                "kind": "base_mismatch",
                "stored": "refs/remotes/upstream/main",
                "requested": { "kind": "ref", "name": "refs/heads/main" }
            })
        );
        assert_eq!(
            serde_json::from_value::<GitCompared<super::super::DiffSummary>>(encoded).unwrap(),
            value
        );
    }
}
