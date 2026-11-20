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
//! - The branch is asked again when it is deleted (`branch_is_lost_freely`):
//!   the inspection and the removal are separate runs, and a commit can
//!   land on the branch between them. If one did, the branch stays (its
//!   folder is gone; the branch keeps the commit).

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

    /// Whether deleting `branch` of `repo` loses no commit: it is gone, or
    /// everything on it is kept elsewhere. A failure to tell is "no", so
    /// the branch stays.
    pub(super) async fn branch_is_lost_freely(&self, repo: &Path, branch: &BranchName) -> bool {
        let counted = match self.has_branch(repo, branch).await {
            Ok(false) => return true,
            Ok(true) => {
                self.count_unique(repo, &[format!("refs/heads/{branch}")], Some(branch))
                    .await
            }
            Err(error) => Err(error),
        };
        match counted {
            Ok(0) => true,
            Ok(commits) => {
                tracing::warn!(%branch, commits, "the worktree's branch has commits of its own; kept");
                false
            }
            Err(error) => {
                tracing::warn!(%branch, %error, "could not count the branch's own commits; kept");
                false
            }
        }
    }

    /// `branch -D` of `branch` in `repo`; one already gone is no error.
    pub(super) async fn delete_branch(
        &self,
        repo: &Path,
        branch: &BranchName,
    ) -> Result<(), GitError> {
        let deleted = self
            .command(repo)
            .args(["branch", "-D", "--end-of-options"])
            .arg(branch.as_str())
            .output()
            .await?;
        if deleted.code != Some(0) {
            tracing::debug!(%branch, "the worktree's branch was already gone");
        }
        Ok(())
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
