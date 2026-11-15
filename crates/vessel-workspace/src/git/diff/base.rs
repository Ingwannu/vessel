//! Paseo checkout-git.ts:1704–1735: unqualified comparison bases prefer
//! origin, while qualified refs retain their exact branch stream. History's
//! separate most-ahead heuristic must not leak into diff selection.

use std::path::Path;

use vessel_proto::workspace::{CommitId, WorktreeBase};

use super::ComparisonError;
use crate::git::{Git, GitError};

const LOCAL: &str = "refs/heads/";
const REMOTE: &str = "refs/remotes/";
const ORIGIN: &str = "refs/remotes/origin/";

impl Git {
    pub(super) async fn comparison_base(
        &self,
        dir: &Path,
        base: &WorktreeBase,
    ) -> Result<CommitId, ComparisonError> {
        match base {
            // Preserve Vessel's explicit-commit extension without resolving a
            // same-named branch or applying a branch selection heuristic.
            WorktreeBase::Commit { .. } => Ok(self
                .resolve(dir, base)
                .await?
                .ok_or(GitError::Unexpected { what: "base" })?),
            WorktreeBase::Ref { name } => {
                let name = name.as_str();
                if name.starts_with(LOCAL) || name.starts_with(REMOTE) {
                    return Ok(self
                        .exact_comparison_ref(dir, name)
                        .await?
                        .ok_or(GitError::Unexpected { what: "base" })?);
                }
                let branch = name.strip_prefix("origin/").unwrap_or(name);
                let origin = format!("{ORIGIN}{branch}");
                let local = format!("{LOCAL}{branch}");
                let (origin, local) = tokio::try_join!(
                    self.exact_comparison_ref(dir, &origin),
                    self.exact_comparison_ref(dir, &local),
                )?;
                Ok(origin
                    .or(local)
                    .ok_or(GitError::Unexpected { what: "base" })?)
            }
        }
    }

    /// Check existence before peeling the ref to a commit: a malformed remote
    /// ref is a failure, not permission to substitute the local branch.
    async fn exact_comparison_ref(
        &self,
        dir: &Path,
        name: &str,
    ) -> Result<Option<CommitId>, ComparisonError> {
        let exists = self
            .command(dir)
            .args(["show-ref", "--verify", "--quiet"])
            .arg(name)
            .output()
            .await?;
        if exists.code == Some(1) && !exists.stdout_cut {
            return Ok(None);
        }
        let _exists = exists.checked()?;
        let commit = self
            .command(dir)
            .args(["rev-parse", "--verify", "--quiet"])
            .arg(format!("{name}^{{commit}}"))
            .output()
            .await?
            .checked()?;
        let text = std::str::from_utf8(&commit.stdout)?;
        Ok(Some(text.trim_end_matches(['\r', '\n']).parse()?))
    }
}
