//! Pending ADR 0087 contract. These types do not register a method or change
//! legacy `DiffScope`: admission belongs to the single Quality B method table.
//!
//! [Decision Log]
//! - Purpose: committed-only must exclude the index and worktree, unlike Base.
//! - Constraint: adding a legacy scope variant could reach an old node through
//!   an already-admitted call. Existing calls must keep their exact meaning.
//! - Choice: a separate typed comparison request, staged until admission.
//! - Effect: new clients can refuse unsupported comparison explicitly; no
//!   patch parsing, command arguments or absolute paths cross this boundary.

use serde::{Deserialize, Serialize};

use super::{CommitId, RelPath, WorkspaceRoot, WorktreeBase};

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
}
