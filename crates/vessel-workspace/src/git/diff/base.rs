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
                if name.as_str().starts_with(LOCAL) || name.as_str().starts_with(REMOTE) {
                    return Ok(self
                        .exact_branch_commit(dir, name)
                        .await?
                        .ok_or(GitError::Unexpected { what: "base" })?);
                }
                let branch = name
                    .as_str()
                    .strip_prefix("origin/")
                    .unwrap_or(name.as_str());
                let origin: vessel_proto::workspace::RefName =
                    format!("{ORIGIN}{branch}").parse()?;
                let local: vessel_proto::workspace::RefName = format!("{LOCAL}{branch}").parse()?;
                let (origin, local) = tokio::try_join!(
                    self.exact_branch_commit(dir, &origin),
                    self.exact_branch_commit(dir, &local),
                )?;
                Ok(origin
                    .or(local)
                    .ok_or(GitError::Unexpected { what: "base" })?)
            }
        }
    }
}
