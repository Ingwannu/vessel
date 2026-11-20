//! Commits a worktree's removal would lose: those reachable from a
//! detached `HEAD` or from the branch the node created, that no other
//! branch, remote-tracking branch, tag or Vessel snapshot has (ADR 0027).
//!
//! [Decision Log]
//! - A count that git could not make is an error, never zero: only the
//!   created branch being gone (renamed or deleted by hand) is "nothing on
//!   it", and that is asked of git directly (`show-ref`) rather than read
//!   from a failed `rev-list`, which also fails on a corrupt object
//!   database or a bad ref.

use std::path::Path;

use vessel_proto::workspace::BranchName;

use super::super::{Git, GitError, tail};
use super::SNAPSHOTS;

impl Git {
    /// Commits reachable from a detached `HEAD` in `dir` or from the
    /// `created` branch that nothing else keeps.
    pub(super) async fn unique_commits(
        &self,
        dir: &Path,
        created: Option<&BranchName>,
    ) -> Result<u32, GitError> {
        let detached = self.status(dir).await?.branch.is_none();
        let created = match created {
            Some(branch) if self.has_branch(dir, branch).await? => Some(branch),
            // Gone (renamed or deleted by hand): nothing of it to lose.
            Some(_) | None => None,
        };
        let mut tips = Vec::new();
        if detached {
            tips.push("HEAD".to_owned());
        }
        if let Some(branch) = created {
            tips.push(format!("refs/heads/{branch}"));
        }
        self.count_unique(dir, &tips, created).await
    }

    /// Commits reachable from `tips` and from no branch but `created`, no
    /// remote-tracking branch, tag or snapshot.
    async fn count_unique(
        &self,
        dir: &Path,
        tips: &[String],
        created: Option<&BranchName>,
    ) -> Result<u32, GitError> {
        if tips.is_empty() {
            return Ok(0);
        }
        let mut command = self
            .command(dir)
            .args(["rev-list", "--count"])
            .args(tips)
            .arg("--not");
        if let Some(branch) = created {
            command = command.arg(format!("--exclude={branch}"));
        }
        let output = command
            .args(["--branches", "--remotes", "--tags"])
            .arg(format!("--glob={SNAPSHOTS}/*"))
            .output()
            .await?;
        if output.code != Some(0) {
            return Err(GitError::Failed {
                code: output.code,
                message: tail(&output.stderr_text()),
            });
        }
        // Unreadable counts as more than any: the worktree stays.
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .unwrap_or(u32::MAX))
    }

    /// Whether `branch` exists in `dir`'s repository.
    async fn has_branch(&self, dir: &Path, branch: &BranchName) -> Result<bool, GitError> {
        let output = self
            .command(dir)
            .args(["show-ref", "--verify", "--quiet"])
            .arg(format!("refs/heads/{branch}"))
            .output()
            .await?;
        match output.code {
            Some(0) => Ok(true),
            // `show-ref --verify` answers 1 for a ref that does not exist.
            Some(1) => Ok(false),
            code => Err(GitError::Failed {
                code,
                message: tail(&output.stderr_text()),
            }),
        }
    }
}
