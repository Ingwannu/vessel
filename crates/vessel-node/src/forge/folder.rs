//! Where a session's pull request would be, read off its folder: the
//! branch and commit checked out, the remote that branch pushes to, and
//! the repository that remote's address names (ADR 0029).
//!
//! [Decision Log]
//! - Purpose: a pull request belongs to a repository and a branch; the
//!   session's folder says both, and changes under the node (an agent
//!   switches branches, adds a remote).
//! - Choice: one git run per cycle (the checkout); the remote's address
//!   only when the branch changed or [`REMOTE_EVERY`] passed, as the
//!   caller's [`Remembered`] keeps it. The address may hold credentials:
//!   only its host and path are kept.
//! - A session is not watched on the project's base branch (no pull
//!   request starts there), on no branch, or with no remote a forge is
//!   named by; each is a reason a card can show.

use std::path::Path;
use std::time::{Duration, Instant};

use vessel_forge::remote;
use vessel_proto::ProjectId;
use vessel_proto::forge::{ForgeAccess, ForgeKind, RepoRef};
use vessel_proto::workspace::{BranchName, CommitId, RemoteName};

use crate::workspace::Workspaces;

/// How often a branch's remote is read again while the branch stays.
const REMOTE_EVERY: Duration = Duration::from_secs(600);

/// Where a session's pull request would be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spot {
    /// The repository its branch's remote names.
    pub(crate) repo: RepoRef,
    /// The branch checked out.
    pub(crate) branch: BranchName,
    /// The commit checked out.
    pub(crate) head: CommitId,
    /// The remote the branch pushes to.
    pub(crate) remote: RemoteName,
}

impl Spot {
    /// What a watch is keyed by.
    pub(crate) fn key(&self) -> Key {
        (self.repo.clone(), self.branch.clone())
    }
}

/// A watch's key: a repository and a branch.
pub(crate) type Key = (RepoRef, BranchName);

/// A branch's remote as last read, so it is not read every cycle.
#[derive(Debug, Clone)]
pub(crate) struct Remembered {
    branch: BranchName,
    found: Option<(RemoteName, RepoRef)>,
    at: Instant,
}

/// Read `folder`, a session's in `project`: where its pull request would
/// be, or why there is none to watch. `remembered` is the last read's
/// remote, given back updated.
pub(crate) async fn read(
    workspaces: &Workspaces,
    folder: &Path,
    project: ProjectId,
    remembered: Option<Remembered>,
) -> (Result<Spot, ForgeAccess>, Option<Remembered>) {
    let checkout = match workspaces.checkout(folder).await {
        Ok(checkout) => checkout,
        Err(error) => {
            tracing::debug!(%error, "a session's folder could not be read for its branch");
            return (Err(ForgeAccess::NoBranch), remembered);
        }
    };
    let (Some(branch), Some(head)) = (checkout.branch, checkout.head) else {
        return (Err(ForgeAccess::NoBranch), remembered);
    };
    if workspaces.base_branch(project).as_ref() == Some(&branch) {
        return (Err(ForgeAccess::OnBase), remembered);
    }
    let fresh = remembered.filter(|kept| kept.branch == branch && kept.at.elapsed() < REMOTE_EVERY);
    let remembered = match fresh {
        Some(kept) => kept,
        None => Remembered {
            found: remote_of(workspaces, folder, &branch).await,
            branch: branch.clone(),
            at: Instant::now(),
        },
    };
    let spot = match &remembered.found {
        Some((remote, repo)) => Ok(Spot {
            repo: repo.clone(),
            branch,
            head,
            remote: remote.clone(),
        }),
        None => Err(ForgeAccess::NoRemote),
    };
    (spot, Some(remembered))
}

/// The remote `branch` pushes to and the repository its address names;
/// `None` when there is none, or its address names no forge (a path).
async fn remote_of(
    workspaces: &Workspaces,
    folder: &Path,
    branch: &BranchName,
) -> Option<(RemoteName, RepoRef)> {
    let (name, address) = match workspaces.remote_address(folder, branch).await {
        Ok(found) => found?,
        Err(error) => {
            tracing::debug!(%error, "a session's remote could not be read");
            return None;
        }
    };
    let parsed = remote::parse(address.expose())?;
    Some((
        name,
        RepoRef {
            kind: ForgeKind::Github,
            host: parsed.host,
            path: parsed.path,
        },
    ))
}
