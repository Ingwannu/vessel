//! Where a new worktree starts: the branch `worktree.create` asks for,
//! resolved against the repository into the branch it checks out, the
//! commit it starts from, the branch the node creates for it and the
//! snapshot to put back. It only reads the repository; making the folder
//! is `create`'s.

use std::path::Path;

use vessel_proto::WorktreeId;
use vessel_proto::workspace::{
    BranchName, WorktreeBase, WorktreeBranch, WorktreeRefusal, WorktreeStart,
};
use vessel_workspace::git::NewBranch;

use crate::workspace::WorkspaceError;
use crate::workspace::projects::short;

/// The branch a new worktree checks out, where it starts, the branch the
/// node creates for it, and the snapshot to put back in it.
pub(super) struct Chosen {
    pub(super) branch: NewBranch,
    pub(super) start: WorktreeStart,
    pub(super) created: Option<BranchName>,
    pub(super) restore: Option<vessel_proto::workspace::CommitId>,
}

/// Resolve what `worktree.create` asks for; a refusal when it names
/// nothing the repository has.
pub(super) async fn choose(
    git: &vessel_workspace::Git,
    root: &Path,
    default_branch: Option<vessel_proto::workspace::RefName>,
    asked: WorktreeBranch,
    id: WorktreeId,
) -> Result<Result<Chosen, WorktreeRefusal>, WorkspaceError> {
    Ok(Ok(match asked {
        WorktreeBranch::New { name, base } => {
            let explicit = base.is_some();
            let base = base.or_else(|| default_branch.map(|name| WorktreeBase::Ref { name }));
            let commit = match &base {
                Some(base) => git.resolve(root, base).await?,
                None => git.head(root).await?,
            };
            let Some(commit) = commit else {
                return Ok(Err(if explicit {
                    WorktreeRefusal::NoSuchBase
                } else {
                    WorktreeRefusal::NoCommit
                }));
            };
            let name = match name {
                Some(name) => name,
                None => branch_for(id)?,
            };
            Chosen {
                start: WorktreeStart {
                    commit: commit.clone(),
                    name: match base {
                        Some(WorktreeBase::Ref { name }) => Some(name),
                        Some(WorktreeBase::Commit { .. }) | None => None,
                    },
                },
                branch: NewBranch::Create {
                    name: name.clone(),
                    start: commit,
                },
                created: Some(name),
                restore: None,
            }
        }
        WorktreeBranch::Existing { name } => {
            let named = WorktreeBase::Ref {
                name: name.as_ref_name(),
            };
            let Some(commit) = git.resolve(root, &named).await? else {
                return Ok(Err(WorktreeRefusal::NoSuchBranch));
            };
            Chosen {
                start: WorktreeStart {
                    commit,
                    name: Some(name.as_ref_name()),
                },
                branch: NewBranch::Existing { name },
                created: None,
                restore: None,
            }
        }
        WorktreeBranch::Snapshot { worktree, name } => {
            let snapshots = git.snapshots(root).await?;
            let Some(snapshot) = snapshots
                .snapshots
                .into_iter()
                .find(|snapshot| snapshot.worktree == worktree)
            else {
                return Ok(Err(WorktreeRefusal::NoSuchSnapshot));
            };
            let name = match name {
                Some(name) => name,
                None => branch_for(id)?,
            };
            Chosen {
                start: WorktreeStart {
                    commit: snapshot.head.clone(),
                    name: None,
                },
                branch: NewBranch::Create {
                    name: name.clone(),
                    start: snapshot.head,
                },
                created: Some(name),
                restore: Some(snapshot.commit),
            }
        }
    }))
}

/// The branch a new worktree gets when none is named.
fn branch_for(worktree: WorktreeId) -> Result<BranchName, WorkspaceError> {
    format!("vessel/{}", short(worktree))
        .parse()
        .map_err(|_never| WorkspaceError::Invalid("no branch name for the worktree".to_owned()))
}
