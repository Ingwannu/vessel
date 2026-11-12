//! What a pull request needs of a folder (protocol 11, ADR 0029): the
//! commit and branch checked out, the remote a branch pushes to and that
//! remote's address, and where the remote was last seen to have the
//! branch. Reads only; the push `pr.open` makes is `write`'s.
//!
//! [Decision Log]
//! - Purpose: the node watches a session's pull request by its folder's
//!   branch and the repository its remote names, every cycle, and opens
//!   one only for a branch the remote has.
//! - Choice: the checkout is one run (`rev-parse HEAD --symbolic-full-name
//!   HEAD`: the commit, then the branch's full name or `HEAD` when
//!   detached), so a watch cycle costs one git run per folder. The
//!   address is `remote get-url`, so a `url.<base>.insteadOf` applies as
//!   it does to the push; it may hold credentials (`https://user:token@`),
//!   so it is kept in [`RemoteAddress`], whose `Debug` shows nothing, and
//!   whoever reads it takes the host and path only.
//! - Where the remote has the branch is the remote-tracking ref, as the
//!   last fetch or push left it: no network, so an agent's `pr.open`
//!   never makes the node reach a remote (the agent pushes with its own
//!   tools, which updates the ref).

use std::fmt;
use std::path::Path;

use vessel_proto::workspace::{BranchName, CommitId, RemoteName};

use super::{Git, GitError};

/// The commit and branch a folder has checked out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkout {
    /// `HEAD`'s commit; `None` before the first commit.
    pub head: Option<CommitId>,
    /// The branch, `None` when detached (or before the first commit).
    pub branch: Option<BranchName>,
}

/// A remote's address, as git would reach it. May hold credentials: its
/// `Debug` shows nothing, and it is never logged or sent.
#[derive(Clone, PartialEq, Eq)]
pub struct RemoteAddress(String);

impl RemoteAddress {
    /// The address, for reading its host and path only.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RemoteAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RemoteAddress(..)")
    }
}

impl Git {
    /// The commit and branch `dir` has checked out, in one run.
    ///
    /// # Errors
    /// As the run's.
    pub async fn checkout(&self, dir: &Path) -> Result<Checkout, GitError> {
        let output = self
            .command(dir)
            .args(["rev-parse", "HEAD", "--symbolic-full-name", "HEAD"])
            .output()
            .await?;
        if output.code != Some(0) {
            return Ok(Checkout {
                head: None,
                branch: None,
            });
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut lines = text.lines().map(str::trim);
        let head = lines.next().and_then(|line| line.parse().ok());
        let branch = lines
            .next()
            .and_then(|line| line.strip_prefix("refs/heads/"))
            .and_then(|name| name.parse().ok());
        Ok(Checkout { head, branch })
    }

    /// The remote `branch` pushes to (its `branch.<name>.remote`, else
    /// `origin`) and that remote's address; `None` when there is no such
    /// remote.
    ///
    /// # Errors
    /// As the runs'.
    pub async fn remote_address(
        &self,
        dir: &Path,
        branch: &BranchName,
    ) -> Result<Option<(RemoteName, RemoteAddress)>, GitError> {
        let remote = self.remote_of_branch(dir, Some(branch)).await;
        let Ok(name) = remote.parse::<RemoteName>() else {
            return Ok(None);
        };
        let output = self
            .command(dir)
            .args(["remote", "get-url", "--"])
            .arg(name.to_string())
            .output()
            .await?;
        if output.code != Some(0) {
            return Ok(None);
        }
        let address = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        Ok((!address.is_empty()).then_some((name, RemoteAddress(address))))
    }

    /// The commit `remote` was last seen to have `branch` at (its
    /// remote-tracking ref); `None` when it has never been seen there.
    ///
    /// # Errors
    /// As the run's.
    pub async fn tracking(
        &self,
        dir: &Path,
        remote: &RemoteName,
        branch: &BranchName,
    ) -> Result<Option<CommitId>, GitError> {
        let output = self
            .command(dir)
            .args(["rev-parse", "--verify", "--quiet"])
            .arg(format!("refs/remotes/{remote}/{branch}^{{commit}}"))
            .output()
            .await?;
        if output.code != Some(0) {
            return Ok(None);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().parse().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Sandbox;

    #[tokio::test]
    async fn a_folder_s_branch_remote_and_tracking_ref_are_read() {
        let sandbox = Sandbox::new();
        let git = sandbox.git().await;
        let empty = sandbox.path().join("empty");
        std::fs::create_dir(&empty).unwrap();
        sandbox.run(&empty, &["init", "-q"]);
        assert_eq!(
            git.checkout(&empty).await.unwrap(),
            Checkout {
                head: None,
                branch: None
            }
        );
        let repo = sandbox.repo("r");
        sandbox.run(&repo, &["switch", "-q", "-c", "topic"]);
        let checkout = git.checkout(&repo).await.unwrap();
        let topic: BranchName = "topic".parse().unwrap();
        assert_eq!(checkout.branch.as_ref(), Some(&topic));
        let head = checkout.head.unwrap();
        assert_eq!(git.remote_address(&repo, &topic).await.unwrap(), None);
        sandbox.run(
            &repo,
            &[
                "remote",
                "add",
                "origin",
                "https://user:secret@example.invalid/o/n.git",
            ],
        );
        let (remote, address) = git.remote_address(&repo, &topic).await.unwrap().unwrap();
        assert_eq!(remote.to_string(), "origin");
        assert_eq!(
            address.expose(),
            "https://user:secret@example.invalid/o/n.git"
        );
        assert!(!format!("{address:?}").contains("secret"));
        assert_eq!(git.tracking(&repo, &remote, &topic).await.unwrap(), None);
        sandbox.run(
            &repo,
            &["update-ref", "refs/remotes/origin/topic", head.as_str()],
        );
        assert_eq!(
            git.tracking(&repo, &remote, &topic).await.unwrap(),
            Some(head.clone())
        );
        sandbox.run(&repo, &["switch", "-q", "--detach"]);
        let detached = git.checkout(&repo).await.unwrap();
        assert_eq!((detached.head, detached.branch), (Some(head), None));
    }
}
