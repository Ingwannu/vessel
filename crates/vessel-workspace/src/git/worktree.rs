//! Worktrees: making one, deciding whether removing one would lose
//! anything, keeping a snapshot, putting a snapshot back, and removing one
//! (ADR 0027).
//!
//! [Decision Log]
//! - Purpose: a session gets a folder and a branch of its own; when it is
//!   done the folder goes, but never with work in it.
//! - "Nothing would be lost" is checked, not assumed: no operation in
//!   progress; no change `git status` shows (submodules included); no
//!   submodule (whose own changes and commits are not checked here); no
//!   entry marked assume-unchanged or skip-worktree (their changes do not
//!   show in a status); no ref of the worktree's own (`refs/worktree/*`,
//!   `refs/bisect/*` go with it); and no commit, reachable from a detached
//!   `HEAD` or from the branch the node created, that no other branch,
//!   remote-tracking branch, tag or Vessel snapshot has (the created branch
//!   is deleted with the worktree; a branch the node did not create stays,
//!   and keeps its commits). GOAT's cleanup checks the same things
//!   (`storageCleanup.ts`, `captureSnapshot.ts`); ignored files are not
//!   counted, as in GOAT's default.
//! - The removal itself is `git worktree remove` without `--force` unless a
//!   person asked to discard or a snapshot was kept: git's own check of a
//!   changed worktree is the last word.
//! - A snapshot (GOAT's `refs/goat/snapshots`, agent-orchestrator's
//!   `refs/ao/preserved`) is one commit of the working tree as `git add -A`
//!   stages it, built through a temporary index so the worktree's own is
//!   untouched, by a fixed identity, its parent `HEAD`, kept at
//!   `refs/vessel/snapshots/<worktree id>`. Refused above 100 MiB for one
//!   file or 256 MiB in all, with an operation in progress, and when it
//!   would hold a nested repository as a bare gitlink.

use std::path::{Path, PathBuf};

use chrono::Utc;
use vessel_proto::WorktreeId;
use vessel_proto::workspace::{
    BranchName, CommitId, GitOperation, KeptReason, Snapshot, SnapshotList,
};

use super::{Git, GitError, refusal};

/// The largest changed file a snapshot takes.
pub(crate) const MAX_SNAPSHOT_FILE: u64 = 100 * 1024 * 1024;
/// The most a snapshot takes in all.
pub(crate) const MAX_SNAPSHOT_TOTAL: u64 = 256 * 1024 * 1024;
/// Where snapshots live in a repository.
const SNAPSHOTS: &str = "refs/vessel/snapshots";
/// The most snapshots listed.
pub(crate) const MAX_SNAPSHOTS: usize = 1000;
/// The mode git gives a submodule or a nested repository.
const GITLINK: &str = "160000";
/// Who makes a snapshot commit: never a guessed user identity.
const IDENTITY: [(&str, &str); 4] = [
    ("GIT_AUTHOR_NAME", "Vessel"),
    ("GIT_AUTHOR_EMAIL", "vessel@localhost"),
    ("GIT_COMMITTER_NAME", "Vessel"),
    ("GIT_COMMITTER_EMAIL", "vessel@localhost"),
];

/// Which branch a new worktree checks out.
#[derive(Debug, Clone)]
pub enum NewBranch {
    /// A new branch at `start`.
    Create {
        /// Its name.
        name: BranchName,
        /// Its first commit.
        start: CommitId,
    },
    /// A branch that exists.
    Existing {
        /// Its name.
        name: BranchName,
    },
}

/// Why `git worktree add` made nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddRefusal {
    /// The new branch exists.
    BranchExists,
    /// The branch is checked out in that folder.
    CheckedOutElsewhere(PathBuf),
    /// No such branch.
    NoSuchBranch,
}

/// Why a snapshot was not made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotRefusal {
    /// An operation is in progress.
    Operation(GitOperation),
    /// A changed file, or all of them, past the bounds.
    TooLarge,
    /// The working tree holds a repository that is not a registered
    /// submodule: it would be kept as a commit id, without its files.
    NestedRepository,
}

impl Git {
    /// `git worktree add` of `branch` at `path` (which must not exist), from
    /// the repository at `repo`. No hook runs.
    ///
    /// # Errors
    /// As the run's; refusals are `Ok(Err(..))`.
    pub async fn add_worktree(
        &self,
        repo: &Path,
        path: &Path,
        branch: &NewBranch,
    ) -> Result<Result<(), AddRefusal>, GitError> {
        let command = self.command(repo).args(["worktree", "add", "--quiet"]);
        let command = match branch {
            NewBranch::Create { name, start } => command
                .arg("-b")
                .arg(name.as_str())
                .arg("--end-of-options")
                .arg(path)
                .arg(start.as_str()),
            NewBranch::Existing { name } => {
                command.arg("--end-of-options").arg(path).arg(name.as_str())
            }
        };
        let output = {
            // Another add in this repository would read this one's
            // half-written entry (`repository_lock`).
            let _turn = self.worktrees.hold(repo).await;
            command.output().await?
        };
        if output.code == Some(0) {
            return Ok(Ok(()));
        }
        Ok(Err(match refusal::known(&output) {
            Some(vessel_proto::workspace::GitRefusal::BranchExists) => AddRefusal::BranchExists,
            Some(vessel_proto::workspace::GitRefusal::CheckedOutElsewhere { path }) => {
                AddRefusal::CheckedOutElsewhere(path)
            }
            Some(vessel_proto::workspace::GitRefusal::NoSuchRef) => AddRefusal::NoSuchBranch,
            _ => {
                return Err(GitError::Failed {
                    code: output.code,
                    message: super::tail(&output.stderr_text()),
                });
            }
        }))
    }

    /// Everything removing the worktree at `dir` (with its `created`
    /// branch) would lose; empty when nothing would be.
    ///
    /// # Errors
    /// As the runs'.
    pub async fn would_lose(
        &self,
        dir: &Path,
        created: Option<&BranchName>,
    ) -> Result<Vec<KeptReason>, GitError> {
        let mut reasons = Vec::new();
        if let Some(operation) = self.operation(dir).await? {
            reasons.push(KeptReason::Operation { operation });
        }
        let status = self
            .command(dir)
            .args([
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                // Never into a submodule's working tree, which would run
                // git there under that repository's own configuration
                // (ADR 0030); a gitlink keeps the worktree anyway
                // (`Submodules`, below).
                "--ignore-submodules=dirty",
            ])
            .output()
            .await?;
        if status.stdout_cut {
            reasons.push(KeptReason::Uncommitted { files: u32::MAX });
        } else {
            let status = status.checked()?;
            let files = porcelain_v1_count(&status.stdout);
            if files > 0 {
                reasons.push(KeptReason::Uncommitted { files });
            }
        }
        let staged = self
            .command(dir)
            .args(["ls-files", "--stage", "-z"])
            .output()
            .await?
            .checked()?;
        if staged
            .stdout
            .split(|byte| *byte == 0)
            .any(|entry| entry.starts_with(GITLINK.as_bytes()))
        {
            reasons.push(KeptReason::Submodules);
        }
        let flags = self
            .command(dir)
            .args(["ls-files", "-v", "-z"])
            .output()
            .await?
            .checked()?;
        if flags.stdout.split(|byte| *byte == 0).any(|entry| {
            entry
                .first()
                .is_some_and(|tag| tag.is_ascii_lowercase() || *tag == b'S')
        }) {
            reasons.push(KeptReason::FlaggedFiles);
        }
        let refs = self
            .command(dir)
            .args([
                "for-each-ref",
                "--format=%(refname)",
                "refs/worktree",
                "refs/bisect",
            ])
            .output()
            .await?
            .checked()?;
        if !refs.stdout.trim_ascii().is_empty() {
            reasons.push(KeptReason::WorktreeRefs);
        }
        let commits = self.unique_commits(dir, created).await?;
        if commits > 0 {
            reasons.push(KeptReason::UniqueCommits { commits });
        }
        Ok(reasons)
    }

    /// Commits reachable from a detached `HEAD` or the `created` branch that
    /// no other branch, remote-tracking branch, tag or snapshot has.
    async fn unique_commits(
        &self,
        dir: &Path,
        created: Option<&BranchName>,
    ) -> Result<u32, GitError> {
        let detached = self.status(dir).await?.branch.is_none();
        let mut tips = Vec::new();
        if detached {
            tips.push("HEAD".to_owned());
        }
        if let Some(branch) = created {
            tips.push(format!("refs/heads/{branch}"));
        }
        if tips.is_empty() {
            return Ok(0);
        }
        let mut command = self
            .command(dir)
            .args(["rev-list", "--count"])
            .args(&tips)
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
            // The created branch is gone (renamed or deleted by hand):
            // count from `HEAD` alone.
            if created.is_some() && !detached {
                return Ok(0);
            }
            return Err(GitError::Failed {
                code: output.code,
                message: super::tail(&output.stderr_text()),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .unwrap_or(u32::MAX))
    }

    /// Keep the working tree of `dir` as a snapshot of worktree `id`.
    ///
    /// # Errors
    /// As the runs'; refusals are `Ok(Err(..))`.
    pub async fn snapshot(
        &self,
        dir: &Path,
        id: WorktreeId,
        scratch: &Path,
    ) -> Result<Result<Snapshot, SnapshotRefusal>, GitError> {
        if let Some(operation) = self.operation(dir).await? {
            return Ok(Err(SnapshotRefusal::Operation(operation)));
        }
        let head = self
            .head(dir)
            .await?
            .ok_or(GitError::Unexpected { what: "head" })?;
        let branch = self.status(dir).await?.branch;
        if !self.changed_files_fit(dir).await? {
            return Ok(Err(SnapshotRefusal::TooLarge));
        }
        let index = self
            .command(dir)
            .args(["rev-parse", "--git-path", "index"])
            .output()
            .await?
            .checked()?;
        let index = dir.join(String::from_utf8_lossy(&index.stdout).trim());
        let temp = scratch.join(format!("snapshot-index-{id}"));
        let copied = {
            let (index, temp) = (index.clone(), temp.clone());
            tokio::task::spawn_blocking(move || std::fs::copy(index, temp))
                .await
                .map_err(|_panicked| GitError::Unexpected { what: "the index" })?
        };
        let _bytes: u64 = copied.map_err(|_io| GitError::Unexpected { what: "the index" })?;
        let result = self.snapshot_through(dir, id, &temp, &head, branch).await;
        let _removed = tokio::fs::remove_file(&temp).await;
        result
    }

    async fn snapshot_through(
        &self,
        dir: &Path,
        id: WorktreeId,
        temp_index: &Path,
        head: &CommitId,
        branch: Option<BranchName>,
    ) -> Result<Result<Snapshot, SnapshotRefusal>, GitError> {
        let with_index = |command: super::GitCommand| command.env("GIT_INDEX_FILE", temp_index);
        with_index(self.command(dir).args(["add", "--all", "--", "."]))
            .run()
            .await?;
        // A gitlink the real index did not have is a nested repository.
        let links = |stdout: &[u8]| -> Vec<Vec<u8>> {
            stdout
                .split(|byte| *byte == 0)
                .filter(|entry| entry.starts_with(GITLINK.as_bytes()))
                .map(<[u8]>::to_vec)
                .collect()
        };
        let staged = with_index(self.command(dir).args(["ls-files", "--stage", "-z"]))
            .output()
            .await?
            .checked()?;
        let before = self
            .command(dir)
            .args(["ls-files", "--stage", "-z"])
            .output()
            .await?
            .checked()?;
        let known = links(&before.stdout);
        if links(&staged.stdout)
            .iter()
            .any(|link| !known.contains(link))
        {
            return Ok(Err(SnapshotRefusal::NestedRepository));
        }
        let tree = with_index(self.command(dir).arg("write-tree"))
            .output()
            .await?
            .checked()?;
        let tree = String::from_utf8_lossy(&tree.stdout).trim().to_owned();
        let subject = match &branch {
            Some(branch) => format!("vessel snapshot of {id} on {branch}"),
            None => format!("vessel snapshot of {id}"),
        };
        let mut command =
            self.command(dir)
                .args(["commit-tree", &tree, "-p", head.as_str(), "-m", &subject]);
        for (name, value) in IDENTITY {
            command = command.env(name, value);
        }
        let commit = command.output().await?.checked()?;
        let commit: CommitId = String::from_utf8_lossy(&commit.stdout)
            .trim()
            .parse()
            .map_err(|_bad| GitError::Unexpected {
                what: "commit-tree",
            })?;
        self.command(dir)
            .args(["update-ref", &format!("{SNAPSHOTS}/{id}"), commit.as_str()])
            .run()
            .await?;
        Ok(Ok(Snapshot {
            worktree: id,
            branch,
            commit,
            head: head.clone(),
            created_at: Utc::now(),
        }))
    }

    /// Whether every changed and untracked file is within the snapshot's
    /// bounds.
    async fn changed_files_fit(&self, dir: &Path) -> Result<bool, GitError> {
        let changed = self
            .command(dir)
            .args([
                "diff",
                "--name-only",
                "-z",
                "--no-ext-diff",
                "--no-textconv",
                "HEAD",
                "--",
            ])
            .output()
            .await?
            .checked()?;
        let untracked = self
            .command(dir)
            .args(["ls-files", "--others", "--exclude-standard", "-z"])
            .output()
            .await?
            .checked()?;
        let files: Vec<PathBuf> = changed
            .stdout
            .split(|byte| *byte == 0)
            .chain(untracked.stdout.split(|byte| *byte == 0))
            .filter(|path| !path.is_empty())
            .map(|path| dir.join(String::from_utf8_lossy(path).as_ref()))
            .collect();
        tokio::task::spawn_blocking(move || {
            let mut total: u64 = 0;
            for file in files {
                let size = std::fs::symlink_metadata(&file).map_or(0, |metadata| metadata.len());
                total = total.saturating_add(size);
                if size > MAX_SNAPSHOT_FILE || total > MAX_SNAPSHOT_TOTAL {
                    return false;
                }
            }
            true
        })
        .await
        .map_err(|_panicked| GitError::Unexpected { what: "file sizes" })
    }

    /// Put snapshot `snapshot` back in the fresh worktree `dir` (made at its
    /// `head`), as uncommitted changes.
    ///
    /// # Errors
    /// As the runs'.
    pub async fn restore_snapshot(&self, dir: &Path, snapshot: &CommitId) -> Result<(), GitError> {
        self.command(dir)
            .args(["read-tree", "-m", "-u", "HEAD", snapshot.as_str()])
            .run()
            .await?;
        self.command(dir)
            .args(["reset", "--quiet"])
            .output()
            .await?
            .checked()
            .map(drop)
    }

    /// A repository's snapshots, newest first.
    ///
    /// # Errors
    /// As the run's.
    pub async fn snapshots(&self, repo: &Path) -> Result<SnapshotList, GitError> {
        let output = self
            .command(repo)
            .args([
                "for-each-ref",
                "--sort=-creatordate",
                "--format=%(refname)%00%(objectname)%00%(parent)%00%(creatordate:unix)%00%(contents:subject)",
                SNAPSHOTS,
            ])
            .output()
            .await?
            .checked()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut snapshots = Vec::new();
        let mut more = false;
        for line in text.lines() {
            let Some(snapshot) = snapshot_record(line) else {
                continue;
            };
            if snapshots.len() >= MAX_SNAPSHOTS {
                more = true;
                break;
            }
            snapshots.push(snapshot);
        }
        Ok(SnapshotList { snapshots, more })
    }

    /// Delete worktree `id`'s snapshot ref; `false` when there was none.
    ///
    /// # Errors
    /// As the run's.
    pub async fn drop_snapshot(&self, repo: &Path, id: WorktreeId) -> Result<bool, GitError> {
        let name = format!("{SNAPSHOTS}/{id}");
        let exists = self
            .command(repo)
            .args(["show-ref", "--verify", "--quiet", &name])
            .output()
            .await?;
        if exists.code != Some(0) {
            return Ok(false);
        }
        self.command(repo)
            .args(["update-ref", "-d", &name])
            .output()
            .await?
            .checked()
            .map(|_| true)
    }

    /// `git worktree remove` of `path`, forced only when asked; then the
    /// `created` branch is deleted, if any.
    ///
    /// # Errors
    /// As the runs'; [`GitError::Failed`] when git refuses a non-forced
    /// removal (it found a change after all).
    pub async fn remove_worktree(
        &self,
        repo: &Path,
        path: &Path,
        created: Option<&BranchName>,
        force: bool,
    ) -> Result<(), GitError> {
        // The removal and the branch's deletion both read the list of
        // worktrees an add may be writing (`repository_lock`).
        let _turn = self.worktrees.hold(repo).await;
        let mut command = self.command(repo).args(["worktree", "remove"]);
        if force {
            command = command.args(["--force", "--force"]);
        }
        command.arg(path).run().await?;
        if let Some(branch) = created {
            let deleted = self
                .command(repo)
                .args(["branch", "-D", "--end-of-options"])
                .arg(branch.as_str())
                .output()
                .await?;
            if deleted.code != Some(0) {
                tracing::debug!(%branch, "the worktree's branch was already gone");
            }
        }
        Ok(())
    }
}

/// Entries of `status --porcelain=v1 -z` (a rename's record is followed
/// by its old path, which is not counted).
fn porcelain_v1_count(output: &[u8]) -> u32 {
    let mut count: u32 = 0;
    let mut fields = output
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    while let Some(field) = fields.next() {
        count = count.saturating_add(1);
        if matches!(field.first(), Some(b'R' | b'C')) {
            let _old = fields.next();
        }
    }
    count
}

fn snapshot_record(line: &str) -> Option<Snapshot> {
    let mut fields = line.split('\0');
    let worktree = fields
        .next()?
        .strip_prefix(SNAPSHOTS)?
        .strip_prefix('/')?
        .parse()
        .ok()?;
    let commit = fields.next()?.parse().ok()?;
    let head = fields.next()?.split(' ').next()?.parse().ok()?;
    let created_at = fields
        .next()?
        .parse::<i64>()
        .ok()
        .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))?;
    let subject = fields.next().unwrap_or_default();
    let branch = subject
        .rsplit_once(" on ")
        .and_then(|(_, branch)| branch.parse().ok());
    Some(Snapshot {
        worktree,
        branch,
        commit,
        head,
        created_at,
    })
}

#[cfg(test)]
mod tests;
