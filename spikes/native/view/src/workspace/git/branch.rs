//! Branch forms and remote-to-local checkout, matching branch-dialog.

use vessel_proto::workspace::{
    Branch, BranchName, InvalidText, RefName, RemoteName, RenameBranch, Switch, WorkspaceRoot,
    WorktreeBase,
};

/// The branch form's purpose; a rename cannot create a branch by mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchMode {
    /// New branch from the current HEAD.
    Create,
    /// Change the current branch's name.
    Rename,
}

/// A prepared branch form; the link sends its own wire type unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchRequest {
    /// Create and switch.
    Create(Switch),
    /// Rename the current branch.
    Rename(RenameBranch),
}

/// Why a listed branch cannot become a local checkout request.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SwitchProblem {
    /// The node supplied a ref whose prefix does not match its remote.
    #[error("branch {reference} does not belong to the listed remote {remote}")]
    RemoteMismatch {
        /// The listed ref.
        reference: RefName,
        /// The remote it claimed.
        remote: RemoteName,
    },
    /// A remote ref's local name is not a valid branch name.
    #[error(transparent)]
    InvalidName(#[from] InvalidText),
}

/// Prepare a create or rename form using the wire's branch validation.
///
/// # Errors
/// The wire's validation error for the typed branch name.
pub fn branch_request(
    root: WorkspaceRoot,
    mode: BranchMode,
    name: &str,
) -> Result<BranchRequest, InvalidText> {
    let name: BranchName = name.trim().parse()?;
    Ok(match mode {
        BranchMode::Create => BranchRequest::Create(Switch {
            root,
            branch: name,
            create: Some(WorktreeBase::Ref {
                name: "HEAD".parse()?,
            }),
        }),
        BranchMode::Rename => BranchRequest::Rename(RenameBranch {
            root,
            from: None,
            to: name,
        }),
    })
}

/// Prepare a listed branch: a remote-tracking ref becomes a local branch
/// starting at that ref, never a checkout of a detached remote ref.
///
/// # Errors
/// The listed remote does not match the ref, or its local name does not
/// satisfy the wire's branch-name rules.
pub fn switch_request(root: WorkspaceRoot, branch: &Branch) -> Result<Switch, SwitchProblem> {
    let (local, create) = match &branch.remote {
        None => (branch.name.as_str(), None),
        Some(remote) => {
            let prefix = format!("{remote}/");
            let local = branch.name.as_str().strip_prefix(&prefix).ok_or_else(|| {
                SwitchProblem::RemoteMismatch {
                    reference: branch.name.clone(),
                    remote: remote.clone(),
                }
            })?;
            (
                local,
                Some(WorktreeBase::Ref {
                    name: branch.name.clone(),
                }),
            )
        }
    };
    Ok(Switch {
        root,
        branch: local.parse()?,
        create,
    })
}
