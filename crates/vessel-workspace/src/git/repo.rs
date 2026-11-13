//! What a repository is: its top level, its commits by name, its default
//! branch, a file at a commit, and a new one.
//!
//! Names given to git here (`RefName`, `CommitId`, `BranchName`) never
//! start with `-` (their types refuse it), so they cannot read as options
//! even where a subcommand predates `--end-of-options` (`rev-parse` takes it
//! only from git 2.30; the minimum is 2.25).

use std::path::{Path, PathBuf};

use vessel_proto::workspace::{CommitId, RefName, WorktreeBase};

use super::{Git, GitError, Limits};

impl Git {
    /// The top level of the repository `dir` is in, or `None` when it is in
    /// none.
    ///
    /// # Errors
    /// As the run's.
    pub async fn toplevel(&self, dir: &Path) -> Result<Option<PathBuf>, GitError> {
        let output = self
            .command(dir)
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .await?;
        if output.code != Some(0) {
            return Ok(None);
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let line = text.trim_end_matches(['\n', '\r']);
        Ok((!line.is_empty()).then(|| PathBuf::from(line)))
    }

    /// Whether `dir` is git's own folder rather than a working tree: a
    /// `.git` folder or a folder inside one, or a bare repository (ADR
    /// 0030). Such a folder is never a project: its `config` and `hooks`
    /// would be files the file calls write.
    ///
    /// # Errors
    /// As the run's.
    pub async fn is_git_dir(&self, dir: &Path) -> Result<bool, GitError> {
        let output = self
            .command(dir)
            .args(["rev-parse", "--is-inside-git-dir", "--is-bare-repository"])
            .output()
            .await;
        let output = match output {
            Ok(output) => output,
            // A repository whose own configuration moves its working tree
            // is as much git's own folder as a bare one.
            Err(GitError::OwnConfig {
                why: super::OwnConfigRefusal::Layout,
                ..
            }) => {
                return Ok(true);
            }
            Err(error) => return Err(error),
        };
        if output.code != Some(0) {
            return Ok(false);
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.trim() == "true"))
    }

    /// The commit `HEAD` names; `None` before the first commit.
    ///
    /// # Errors
    /// As the run's.
    pub async fn head(&self, dir: &Path) -> Result<Option<CommitId>, GitError> {
        self.commit_of(dir, "HEAD").await
    }

    /// The full id of the commit `revision` names; `None` when it names none.
    async fn commit_of(&self, dir: &Path, revision: &str) -> Result<Option<CommitId>, GitError> {
        let output = self
            .command(dir)
            .args(["rev-parse", "--verify", "--quiet"])
            .arg(format!("{revision}^{{commit}}"))
            .output()
            .await?;
        if output.code != Some(0) {
            return Ok(None);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().parse().ok())
    }

    /// The commit `base` names, in full; `None` when it names none. A commit
    /// id must resolve to a commit whose id starts with the digits given, so
    /// a branch named like an id is never taken for it.
    ///
    /// # Errors
    /// As the run's.
    pub async fn resolve(
        &self,
        dir: &Path,
        base: &WorktreeBase,
    ) -> Result<Option<CommitId>, GitError> {
        match base {
            WorktreeBase::Ref { name } => self.commit_of(dir, name.as_str()).await,
            WorktreeBase::Commit { id } => Ok(self
                .commit_of(dir, id.as_str())
                .await?
                .filter(|found| found.as_str().starts_with(id.as_str()))),
        }
    }

    /// The branch new worktrees start from: the remote-tracking branch
    /// `origin/HEAD` names, else a local `main`, else `master`, else the
    /// branch checked out. agent-orchestrator prefers the remote's too, so a
    /// worktree starts from what was last fetched, not a stale local copy.
    ///
    /// # Errors
    /// As the runs'.
    pub async fn default_branch(&self, dir: &Path) -> Result<Option<RefName>, GitError> {
        let remote = self
            .command(dir)
            .args([
                "symbolic-ref",
                "--quiet",
                "--short",
                "refs/remotes/origin/HEAD",
            ])
            .output()
            .await?;
        if remote.code == Some(0)
            && let Ok(name) = String::from_utf8_lossy(&remote.stdout).trim().parse()
        {
            return Ok(Some(name));
        }
        for local in ["main", "master"] {
            let found = self
                .command(dir)
                .args(["show-ref", "--verify", "--quiet"])
                .arg(format!("refs/heads/{local}"))
                .output()
                .await?;
            if found.code == Some(0) {
                return Ok(local.parse().ok());
            }
        }
        let current = self
            .command(dir)
            .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
            .output()
            .await?;
        Ok((current.code == Some(0))
            .then(|| String::from_utf8_lossy(&current.stdout).trim().parse().ok())
            .flatten())
    }

    /// The bytes of `path` at `commit` (`git cat-file blob`, never a
    /// checkout: no filter or text conversion runs), at most `max` of them;
    /// `None` when there is no such file. [`GitError::OutputLimit`] when it
    /// is larger.
    ///
    /// # Errors
    /// As the run's.
    pub async fn read_blob(
        &self,
        dir: &Path,
        commit: &CommitId,
        path: &str,
        max: usize,
    ) -> Result<Option<Vec<u8>>, GitError> {
        let output = self
            .command(dir)
            .args(["cat-file", "blob"])
            .arg(format!("{commit}:{path}"))
            .limits(Limits::LOCAL.with_stdout(max))
            .output()
            .await?;
        if output.stdout_cut {
            return Err(GitError::OutputLimit);
        }
        if output.code != Some(0) {
            return Ok(None);
        }
        Ok(Some(output.stdout))
    }

    /// A new repository in `dir` (which exists).
    ///
    /// # Errors
    /// As the run's, and [`GitError::Failed`] when git refuses.
    pub async fn init(&self, dir: &Path) -> Result<(), GitError> {
        self.command(dir)
            .outside_repository()
            .args(["init", "--quiet"])
            .output()
            .await?
            .checked()
            .map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Sandbox;

    #[tokio::test]
    async fn names_resolve_and_a_branch_named_like_an_id_is_not_an_id() {
        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        let git = sandbox.git().await;
        assert_eq!(git.toplevel(&repo.join(".git")).await.unwrap(), None);
        let top = git.toplevel(&repo).await.unwrap().unwrap();
        assert_eq!(
            std::fs::canonicalize(top).unwrap(),
            std::fs::canonicalize(&repo).unwrap()
        );
        assert_eq!(git.toplevel(sandbox.path()).await.unwrap(), None);
        let head = git.head(&repo).await.unwrap().unwrap();
        assert_eq!(head.as_str().len(), 40);
        let short: CommitId = head.as_str().get(..7).unwrap().parse().unwrap();
        assert_eq!(
            git.resolve(&repo, &WorktreeBase::Commit { id: short })
                .await
                .unwrap(),
            Some(head.clone())
        );
        // A branch named like seven hex digits of no commit.
        sandbox.run(&repo, &["branch", "abcdef1"]);
        let fake: CommitId = "abcdef1".parse().unwrap();
        assert_eq!(
            git.resolve(&repo, &WorktreeBase::Commit { id: fake })
                .await
                .unwrap(),
            None
        );
        let main = WorktreeBase::Ref {
            name: "main".parse().unwrap(),
        };
        assert_eq!(git.resolve(&repo, &main).await.unwrap(), Some(head));
        assert_eq!(
            git.default_branch(&repo).await.unwrap().unwrap().as_str(),
            "main"
        );
    }

    #[tokio::test]
    async fn a_blob_is_read_without_a_checkout_and_bounded() {
        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        let git = sandbox.git().await;
        let head = git.head(&repo).await.unwrap().unwrap();
        assert_eq!(
            git.read_blob(&repo, &head, "README.md", 100).await.unwrap(),
            Some(b"hello\n".to_vec())
        );
        assert_eq!(
            git.read_blob(&repo, &head, "nope", 100).await.unwrap(),
            None
        );
        assert!(matches!(
            git.read_blob(&repo, &head, "README.md", 3).await,
            Err(GitError::OutputLimit)
        ));
        let fresh = sandbox.path().join("fresh");
        std::fs::create_dir(&fresh).unwrap();
        git.init(&fresh).await.unwrap();
        assert_eq!(git.head(&fresh).await.unwrap(), None, "no commit yet");
    }
}
