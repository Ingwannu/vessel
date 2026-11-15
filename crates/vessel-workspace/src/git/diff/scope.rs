//! What a diff scope compares: the two sides as git's arguments, the
//! commit compared against, and whether untracked files count. It runs
//! only the reads that resolve a commit; no diff is run here.

use std::path::Path;

use vessel_proto::workspace::{CommitId, DiffScope, WorktreeBase};

use super::Resolved;
use crate::git::{Git, GitError};

impl Git {
    /// ADR 0087's staged comparison resolver. Quality B wiring removes this
    /// staging flag when the new read methods are admitted; old calls cannot
    /// accidentally send a new enum variant to a peer that cannot decode it.
    #[cfg(test)]
    pub(crate) async fn resolve_comparison(
        &self,
        dir: &Path,
        comparison: &vessel_proto::workspace::GitComparison,
        base: Option<&WorktreeBase>,
    ) -> Result<Option<Resolved>, GitError> {
        use vessel_proto::workspace::GitComparison;

        let scope = match comparison {
            GitComparison::Uncommitted => DiffScope::Uncommitted,
            GitComparison::All { base: asked } => {
                if asked.as_ref().or(base).is_none() {
                    return Ok(None);
                }
                DiffScope::Base {
                    base: asked.clone(),
                }
            }
            GitComparison::Commit { id } => DiffScope::Commit { id: id.clone() },
            GitComparison::Committed { base: asked } => {
                let Some(base) = asked.as_ref().or(base) else {
                    return Ok(None);
                };
                let commit = self
                    .resolve(dir, base)
                    .await?
                    .ok_or(GitError::Unexpected { what: "base" })?;
                let head = self.head(dir).await?;
                let Some(head) = head else {
                    let empty = self.empty_tree(dir).await?;
                    return Ok(Some(Resolved {
                        sides: vec![empty.clone(), empty],
                        from: None,
                        untracked: false,
                        index: None,
                    }));
                };
                let fork = self.merge_base(dir, &commit).await?.unwrap_or(commit);
                return Ok(Some(Resolved {
                    sides: vec![fork.to_string(), head.to_string()],
                    from: Some(fork),
                    untracked: false,
                    index: None,
                }));
            }
        };
        self.resolve_scope(dir, &scope, base).await.map(Some)
    }

    /// What `scope` compares in the repository at `dir`. `base` is the
    /// base to use when the scope names none (a worktree's own, or the
    /// project's default branch).
    ///
    /// # Errors
    /// [`GitError::Unexpected`] (`"base"`) when a base or a commit does not
    /// resolve, and as the runs'.
    pub async fn resolve_scope(
        &self,
        dir: &Path,
        scope: &DiffScope,
        base: Option<&WorktreeBase>,
    ) -> Result<Resolved, GitError> {
        let head = self.head(dir).await?;
        let empty = || async { self.empty_tree(dir).await };
        Ok(match scope {
            DiffScope::Unstaged => Resolved {
                sides: Vec::new(),
                from: head,
                untracked: true,
                index: None,
            },
            DiffScope::Staged => Resolved {
                sides: vec!["--cached".to_owned()],
                from: head,
                untracked: false,
                index: None,
            },
            DiffScope::Uncommitted => {
                let side = match &head {
                    Some(head) => head.to_string(),
                    None => empty().await?,
                };
                Resolved {
                    sides: vec![side],
                    from: head,
                    untracked: true,
                    index: None,
                }
            }
            DiffScope::Base { base: asked } => {
                let base = asked
                    .as_ref()
                    .or(base)
                    .ok_or(GitError::Unexpected { what: "base" })?;
                let commit = self
                    .resolve(dir, base)
                    .await?
                    .ok_or(GitError::Unexpected { what: "base" })?;
                let fork = self.merge_base(dir, &commit).await?.unwrap_or(commit);
                Resolved {
                    sides: vec![fork.to_string()],
                    from: Some(fork),
                    untracked: true,
                    index: None,
                }
            }
            DiffScope::Commit { id } => {
                let commit = self
                    .resolve(dir, &WorktreeBase::Commit { id: id.clone() })
                    .await?
                    .ok_or(GitError::Unexpected { what: "base" })?;
                let parent = self.parent(dir, &commit).await?;
                let from = match &parent {
                    Some(parent) => parent.to_string(),
                    None => empty().await?,
                };
                Resolved {
                    sides: vec![from, commit.to_string()],
                    from: parent,
                    untracked: false,
                    index: None,
                }
            }
        })
    }

    /// The first parent of `commit`; `None` for a root commit.
    async fn parent(&self, dir: &Path, commit: &CommitId) -> Result<Option<CommitId>, GitError> {
        let output = self
            .command(dir)
            .args(["rev-parse", "--verify", "--quiet"])
            .arg(format!("{commit}^1^{{commit}}"))
            .output()
            .await?;
        if output.code != Some(0) {
            return Ok(None);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().parse().ok())
    }

    /// Where `HEAD` and `other` forked; `None` when they share no history.
    async fn merge_base(&self, dir: &Path, other: &CommitId) -> Result<Option<CommitId>, GitError> {
        let output = self
            .command(dir)
            .args(["merge-base", "--end-of-options", "HEAD"])
            .arg(other.as_str())
            .output()
            .await?;
        if output.code != Some(0) {
            return Ok(None);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().parse().ok())
    }

    /// The empty tree's id in this repository's hash (SHA-1 or SHA-256):
    /// the side to compare against before the first commit.
    async fn empty_tree(&self, dir: &Path) -> Result<String, GitError> {
        let output = self
            .command(dir)
            .args(["hash-object", "-t", "tree", "--stdin"])
            .output()
            .await?
            .checked()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }
}
