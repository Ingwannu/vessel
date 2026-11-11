//! What changes a repository: stage, unstage, discard, commit, fetch,
//! push, pull, switch, rename a branch, the stash (protocol 10).
//!
//! Each change checks what it can before it runs (an operation in
//! progress, a detached `HEAD`, an identity, something to commit), so the
//! common refusals are known, not read from git's text; what git refuses
//! after that is read once by `refusal`. Only commit and push run the
//! repository's hooks, as the user's own git would (ADR 0023).

use std::path::Path;

use vessel_proto::workspace::{
    BranchName, CommitInfo, CommitMessage, CommitOutcome, CommitStage, FetchOutcome, FileState,
    GitOperation, GitRefusal, PathSet, PullOutcome, PushOutcome, RemoteName, RenameOutcome,
    StashAction, StashOutcome, SwitchOutcome, WorktreeBase,
};

use super::{Git, GitError, GitOutput, refusal};

/// The most paths one call names.
pub(crate) const MAX_PATHS: usize = 1000;

/// A call's paths, refused past [`MAX_PATHS`].
fn named(paths: &PathSet) -> Result<Option<Vec<&str>>, GitError> {
    match paths {
        PathSet::All => Ok(None),
        PathSet::Paths { paths } if paths.len() > MAX_PATHS => {
            Err(GitError::Unexpected { what: "paths" })
        }
        PathSet::Paths { paths } => Ok(Some(
            paths
                .iter()
                .map(vessel_proto::workspace::RelPath::as_str)
                .collect(),
        )),
    }
}

impl Git {
    /// Stage `paths` (new, changed and removed files alike).
    ///
    /// # Errors
    /// As the run's; [`GitError::Failed`] when git refuses (an index lock).
    pub async fn stage(&self, dir: &Path, paths: &PathSet) -> Result<(), GitError> {
        let command = self.command(dir).args(["add", "--all", "--"]);
        let command = match named(paths)? {
            Some(paths) => command.args(paths),
            None => command.arg("."),
        };
        command.output().await?.checked().map(drop)
    }

    /// Unstage `paths`, keeping their changes in the working tree.
    ///
    /// # Errors
    /// As [`stage`](Self::stage).
    pub async fn unstage(&self, dir: &Path, paths: &PathSet) -> Result<(), GitError> {
        let named = named(paths)?;
        let command = if self.head(dir).await?.is_some() {
            self.command(dir).args(["restore", "--staged", "--"])
        } else {
            // Before the first commit there is no `HEAD` to restore from.
            self.command(dir)
                .args(["rm", "--cached", "-r", "-q", "--ignore-unmatch", "--"])
        };
        let command = match named {
            Some(paths) => command.args(paths),
            None => command.arg("."),
        };
        command.output().await?.checked().map(drop)
    }

    /// Restore `paths` from `HEAD` (index and working tree); with
    /// `untracked`, delete the untracked ones among them too. Ignored files
    /// are never touched.
    ///
    /// # Errors
    /// [`GitError::Unexpected`] (`"head"`) before the first commit, and as
    /// the runs'.
    pub async fn discard(
        &self,
        dir: &Path,
        paths: &PathSet,
        untracked: bool,
    ) -> Result<(), GitError> {
        if self.head(dir).await?.is_none() {
            return Err(GitError::Unexpected { what: "head" });
        }
        let named = named(paths)?;
        let status = self.status(dir).await?;
        let wanted = |path: &str| named.as_ref().is_none_or(|paths| paths.contains(&path));
        let mut tracked = Vec::new();
        let mut new = Vec::new();
        for entry in &status.files {
            if !wanted(entry.path.as_str()) {
                continue;
            }
            if entry.worktree == FileState::Untracked {
                new.push(entry.path.as_str().to_owned());
            } else {
                tracked.push(entry.path.as_str().to_owned());
                if let Some(old) = &entry.old_path {
                    tracked.push(old.as_str().to_owned());
                }
            }
        }
        if !tracked.is_empty() {
            self.command(dir)
                .args(["restore", "--source=HEAD", "--staged", "--worktree", "--"])
                .args(&tracked)
                .run()
                .await?;
        }
        if untracked && !new.is_empty() {
            self.command(dir)
                .args(["clean", "-f", "-q", "--"])
                .args(&new)
                .run()
                .await?;
        }
        Ok(())
    }

    /// Commit what `stage` says, with the repository's identity and hooks.
    ///
    /// # Errors
    /// As the runs'; refusals are outcomes.
    pub async fn commit(
        &self,
        dir: &Path,
        message: &CommitMessage,
        stage: CommitStage,
    ) -> Result<CommitOutcome, GitError> {
        let refused = |refusal| Ok(CommitOutcome::Refused { refusal });
        if let Some(operation @ (GitOperation::Rebase | GitOperation::Bisect)) =
            self.operation(dir).await?
        {
            return refused(GitRefusal::Operation { operation });
        }
        let identity = self
            .command(dir)
            .args([
                "-c",
                "user.useConfigOnly=true",
                "var",
                "GIT_COMMITTER_IDENT",
            ])
            .output()
            .await?;
        if identity.code != Some(0) {
            return refused(GitRefusal::NoIdentity);
        }
        match stage {
            CommitStage::Staged => {}
            CommitStage::Tracked => self.add(dir, "--update").await?,
            CommitStage::Everything => self.add(dir, "--all").await?,
        }
        let staged = self
            .command(dir)
            .args(["diff", "--cached", "--quiet", "--no-ext-diff"])
            .output()
            .await?;
        if staged.code == Some(0) && self.operation(dir).await? != Some(GitOperation::Merge) {
            return refused(GitRefusal::NothingToCommit);
        }
        let output = self
            .command(dir)
            .args(["-c", "user.useConfigOnly=true", "commit", "--quiet", "-m"])
            .arg(message.as_str())
            .with_hooks()
            .output()
            .await;
        let output = match output {
            Ok(output) => output,
            Err(error) => {
                return refusal::of_error(error).map(|refusal| CommitOutcome::Refused { refusal });
            }
        };
        if output.code != Some(0) {
            // Identity and content were checked: what is left is a hook.
            return refused(
                refusal::known(&output).unwrap_or_else(|| GitRefusal::HookFailed {
                    output: super::redact::tail(&output.stderr_text()),
                }),
            );
        }
        let commit = self.last_commit(dir).await?;
        Ok(CommitOutcome::Committed { commit })
    }

    async fn add(&self, dir: &Path, how: &str) -> Result<(), GitError> {
        self.command(dir)
            .args(["add", how, "--", "."])
            .output()
            .await?
            .checked()
            .map(drop)
    }

    /// The commit `HEAD` names, described.
    async fn last_commit(&self, dir: &Path) -> Result<CommitInfo, GitError> {
        self.log(dir, None, 0, Some(1))
            .await?
            .commits
            .into_iter()
            .next()
            .ok_or(GitError::Unexpected {
                what: "the new commit",
            })
    }

    /// The remote of the branch checked out, else `origin`.
    async fn remote_of_branch(&self, dir: &Path, branch: Option<&BranchName>) -> String {
        if let Some(branch) = branch
            && let Ok(output) = self
                .command(dir)
                .args(["config", "--get"])
                .arg(format!("branch.{branch}.remote"))
                .output()
                .await
            && output.code == Some(0)
        {
            let remote = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if remote.parse::<RemoteName>().is_ok() {
                return remote;
            }
        }
        "origin".to_owned()
    }

    /// Fetch `remote` (the branch's, else `origin`), pruning nothing.
    ///
    /// # Errors
    /// As the run's; refusals are outcomes.
    pub async fn fetch(
        &self,
        dir: &Path,
        remote: Option<&RemoteName>,
    ) -> Result<FetchOutcome, GitError> {
        let remote = if let Some(remote) = remote {
            remote.to_string()
        } else {
            let branch = self.status(dir).await?.branch;
            self.remote_of_branch(dir, branch.as_ref()).await
        };
        let output = self
            .command(dir)
            .args([
                "fetch",
                "--quiet",
                "--no-recurse-submodules",
                "--end-of-options",
            ])
            .arg(remote)
            .network()
            .output()
            .await;
        match output {
            Ok(output) if output.code == Some(0) => Ok(FetchOutcome::Fetched),
            Ok(output) => Ok(FetchOutcome::Refused {
                refusal: refusal::of(&output),
            }),
            Err(error) => refusal::of_error(error).map(|refusal| FetchOutcome::Refused { refusal }),
        }
    }

    /// Push the branch checked out: to its upstream, or to `remote` (default
    /// `origin`) when asked to set the upstream or given a remote. Never
    /// forced.
    ///
    /// # Errors
    /// As the run's; refusals are outcomes.
    pub async fn push(
        &self,
        dir: &Path,
        remote: Option<&RemoteName>,
        set_upstream: bool,
    ) -> Result<PushOutcome, GitError> {
        let status = self.status(dir).await?;
        let Some(branch) = status.branch else {
            return Ok(PushOutcome::Refused {
                refusal: GitRefusal::Detached,
            });
        };
        let explicit = set_upstream || remote.is_some();
        if !explicit && status.upstream.is_none() {
            return Ok(PushOutcome::Refused {
                refusal: GitRefusal::NoUpstream,
            });
        }
        let target = match remote {
            Some(remote) => remote.to_string(),
            None if explicit => "origin".to_owned(),
            None => self.remote_of_branch(dir, Some(&branch)).await,
        };
        let mut command = self.command(dir).args(["push", "--quiet"]);
        if explicit {
            if set_upstream {
                command = command.arg("--set-upstream");
            }
            command = command
                .arg("--end-of-options")
                .arg(&target)
                .arg(format!("HEAD:refs/heads/{branch}"));
        }
        let output = command.with_hooks().network().output().await;
        let output = match output {
            Ok(output) => output,
            Err(error) => {
                return refusal::of_error(error).map(|refusal| PushOutcome::Refused { refusal });
            }
        };
        if output.code != Some(0) {
            return Ok(PushOutcome::Refused {
                refusal: refusal::of(&output),
            });
        }
        let remote = target
            .parse()
            .map_err(|_bad| GitError::Unexpected { what: "remote" })?;
        Ok(PushOutcome::Pushed {
            remote,
            branch,
            upstream_set: set_upstream,
        })
    }

    /// Fast-forward the branch checked out from its upstream.
    ///
    /// # Errors
    /// As the runs'; refusals are outcomes.
    pub async fn pull(&self, dir: &Path) -> Result<PullOutcome, GitError> {
        let status = self.status(dir).await?;
        let refused = |refusal| Ok(PullOutcome::Refused { refusal });
        if status.branch.is_none() {
            return refused(GitRefusal::Detached);
        }
        if let Some(operation) = status.operation {
            return refused(GitRefusal::Operation { operation });
        }
        if status.upstream.is_none() {
            return refused(GitRefusal::NoUpstream);
        }
        let before = self.head(dir).await?;
        let output = self
            .command(dir)
            .args([
                "pull",
                "--quiet",
                "--ff-only",
                "--no-rebase",
                "--no-recurse-submodules",
            ])
            .network()
            .output()
            .await;
        let output = match output {
            Ok(output) => output,
            Err(error) => {
                return refusal::of_error(error).map(|refusal| PullOutcome::Refused { refusal });
            }
        };
        if output.code != Some(0) {
            return refused(refusal::of(&output));
        }
        let after = self.head(dir).await?;
        Ok(match (before, after) {
            (Some(from), Some(to)) if from != to => PullOutcome::Pulled { from, to },
            (None, Some(to)) => PullOutcome::Pulled {
                from: to.clone(),
                to,
            },
            _ => PullOutcome::UpToDate,
        })
    }

    /// Check `branch` out, creating it from `create` when given.
    ///
    /// # Errors
    /// As the runs'; refusals are outcomes.
    pub async fn switch(
        &self,
        dir: &Path,
        branch: &BranchName,
        create: Option<&WorktreeBase>,
    ) -> Result<SwitchOutcome, GitError> {
        let mut command = self.command(dir).args(["switch", "--quiet", "--no-guess"]);
        match create {
            Some(start) => {
                let Some(commit) = self.resolve(dir, start).await? else {
                    return Ok(SwitchOutcome::Refused {
                        refusal: GitRefusal::NoSuchRef,
                    });
                };
                command = command
                    .arg("--create")
                    .arg(branch.as_str())
                    .arg(commit.as_str());
            }
            None => command = command.arg(branch.as_str()),
        }
        let output = command.output().await?;
        if output.code != Some(0) {
            return Ok(SwitchOutcome::Refused {
                refusal: refusal::of(&output),
            });
        }
        Ok(SwitchOutcome::Switched {
            branch: branch.clone(),
        })
    }

    /// Rename `from` (the branch checked out when absent) to `to`.
    ///
    /// # Errors
    /// As the runs'; refusals are outcomes.
    pub async fn rename_branch(
        &self,
        dir: &Path,
        from: Option<&BranchName>,
        to: &BranchName,
    ) -> Result<RenameOutcome, GitError> {
        let from = match from {
            Some(from) => from.clone(),
            None => match self.status(dir).await?.branch {
                Some(branch) => branch,
                None => {
                    return Ok(RenameOutcome::Refused {
                        refusal: GitRefusal::Detached,
                    });
                }
            },
        };
        let output = self
            .command(dir)
            .args(["branch", "--move"])
            .arg(from.as_str())
            .arg(to.as_str())
            .output()
            .await?;
        if output.code != Some(0) {
            return Ok(RenameOutcome::Refused {
                refusal: refusal::of(&output),
            });
        }
        Ok(RenameOutcome::Renamed { branch: to.clone() })
    }

    /// Save, apply, pop or drop a stash entry. An entry is named by its
    /// index and its commit: the stash is shared by every worktree of a
    /// repository, so an index another run moved is refused, not applied.
    ///
    /// # Errors
    /// As the runs'; refusals are outcomes.
    pub async fn stash(&self, dir: &Path, action: &StashAction) -> Result<StashOutcome, GitError> {
        let output = match action {
            StashAction::Save { message, untracked } => {
                let before = self
                    .stashes(dir)
                    .await?
                    .entries
                    .first()
                    .map(|entry| entry.commit.clone());
                let mut command = self.command(dir).args(["stash", "push", "--quiet"]);
                if *untracked {
                    command = command.arg("--include-untracked");
                }
                if let Some(message) = message {
                    command = command.arg("--message").arg(message.as_str());
                }
                let output = command.output().await?;
                let after = self
                    .stashes(dir)
                    .await?
                    .entries
                    .first()
                    .map(|entry| entry.commit.clone());
                if output.code == Some(0) && before == after {
                    return Ok(StashOutcome::Refused {
                        refusal: GitRefusal::NothingToStash,
                    });
                }
                output
            }
            StashAction::Apply { index, commit }
            | StashAction::Pop { index, commit }
            | StashAction::Drop { index, commit } => {
                let entry = format!("stash@{{{index}}}");
                let at = self
                    .command(dir)
                    .args(["rev-parse", "--verify", "--quiet"])
                    .arg(&entry)
                    .output()
                    .await?;
                let found = String::from_utf8_lossy(&at.stdout).trim().to_owned();
                if at.code != Some(0) || !found.starts_with(commit.as_str()) {
                    return Ok(StashOutcome::Refused {
                        refusal: GitRefusal::StashMoved,
                    });
                }
                let verb = match action {
                    StashAction::Apply { .. } => "apply",
                    StashAction::Pop { .. } => "pop",
                    StashAction::Drop { .. } | StashAction::Save { .. } => "drop",
                };
                self.command(dir)
                    .args(["stash", verb, "--quiet"])
                    .arg(&entry)
                    .output()
                    .await?
            }
        };
        stash_outcome(self, dir, &output).await
    }
}

async fn stash_outcome(
    git: &Git,
    dir: &Path,
    output: &GitOutput,
) -> Result<StashOutcome, GitError> {
    if output.code != Some(0) {
        return Ok(StashOutcome::Refused {
            refusal: refusal::of(output),
        });
    }
    Ok(StashOutcome::Done {
        stashes: git.stashes(dir).await?,
    })
}

#[cfg(test)]
mod tests;
