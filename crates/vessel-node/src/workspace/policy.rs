//! Who may call what, and which folders may be roots.
//!
//! [Decision Log]
//! - Agents (ADR 0013) already read, write and run git in their own folder
//!   with their own tools; through the node they get only what
//!   orchestration needs: the projects, what a project's file would run,
//!   and a worktree (with its setup output) for a session they start. The
//!   rest refuses them here, before the call runs, whatever their scopes.
//!   An agent never approves commands, so a worktree it makes runs setup
//!   only when a person approved that set.
//! - Checked by method in one `matches!`, so the rule reads as a list; a
//!   new workspace method must be added to it (or it is open to agents
//!   under their scopes, as every other method is).
//! - Roots: a project's root is everything a reader of it can read. A
//!   file system's root, the user's home folder and any folder holding the
//!   node's data folder are refused, for every caller.

use std::path::Path;

use vessel_proto::{ErrorCode, Method, WireError};

use super::{Places, WorkspaceError};
use crate::principal::Principal;

/// `forbidden` for an agent calling a workspace method agents may not.
pub(crate) fn refuse_agents(principal: &Principal, method: Method) -> Result<(), WireError> {
    if principal.agent().is_none() || !refused(method) {
        return Ok(());
    }
    Err(WireError::new(
        ErrorCode::Forbidden,
        format!(
            "agents may not call `{}`: an agent uses its own file and git tools; through the \
             node it may list projects and start a session in a worktree",
            method.as_str()
        ),
    ))
}

/// The workspace methods an agent may not call.
const fn refused(method: Method) -> bool {
    matches!(
        method,
        Method::AddProject
            | Method::CloneProject
            | Method::InitProject
            | Method::RemoveProject
            | Method::ApproveCommands
            | Method::RunSetup
            | Method::RemoveWorktree
            | Method::ListSnapshots
            | Method::DropSnapshot
            | Method::GitStatus
            | Method::GitChanges
            | Method::GitDiff
            | Method::GitShow
            | Method::GitBranches
            | Method::GitLog
            | Method::GitStashes
            | Method::GitStage
            | Method::GitUnstage
            | Method::GitDiscard
            | Method::GitCommit
            | Method::GitFetch
            | Method::GitPush
            | Method::GitPull
            | Method::GitSwitch
            | Method::GitRenameBranch
            | Method::GitStash
            | Method::ListDir
            | Method::ReadFile
            | Method::WriteFile
            | Method::CreateDir
            | Method::RenamePath
            | Method::DeletePath
            | Method::FindFiles
            | Method::SearchFiles
    )
}

/// Refuse `root` (resolved) as a project's root when it would widen what
/// its readers can reach.
pub(super) fn check_root(places: &Places, root: &Path) -> Result<(), WorkspaceError> {
    let refuse = |why: &str| {
        Err(WorkspaceError::Invalid(format!(
            "this folder cannot be a project: {why}"
        )))
    };
    if root.parent().is_none() {
        return refuse("it is a file system's root");
    }
    if root == places.home {
        return refuse("it is the home folder, which holds credentials");
    }
    if places.data.starts_with(root) {
        return refuse("it holds the node's data folder and its token");
    }
    if root.starts_with(&places.data) {
        return refuse("it is inside the node's data folder");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vessel_proto::{NodeId, Scope, SessionId};

    use super::*;

    #[test]
    fn agents_get_orchestration_and_nothing_else() {
        let agent = Principal::Agent {
            node: NodeId::random(),
            session: SessionId::random(),
            scopes: vec![Scope::Read, Scope::Operate],
        };
        for allowed in [
            Method::ListProjects,
            Method::GetProject,
            Method::ProjectCommands,
            Method::ListWorktrees,
            Method::CreateWorktree,
            Method::SetupOutput,
            Method::CreateSession,
        ] {
            assert!(refuse_agents(&agent, allowed).is_ok(), "{allowed:?}");
        }
        for refused in [
            Method::ApproveCommands,
            Method::ReadFile,
            Method::GitPush,
            Method::WriteFile,
        ] {
            let error = refuse_agents(&agent, refused).unwrap_err();
            assert_eq!(error.code, ErrorCode::Forbidden);
        }
        assert!(refuse_agents(&Principal::Owner, Method::ApproveCommands).is_ok());
    }

    #[test]
    fn roots_that_would_reach_the_token_or_the_home_are_refused() {
        let places = Places {
            data: PathBuf::from("/home/me/.vessel"),
            home: PathBuf::from("/home/me"),
        };
        for refused in [
            "/",
            "/home/me",
            "/home",
            "/home/me/.vessel",
            "/home/me/.vessel/workspace",
        ] {
            assert!(
                check_root(&places, Path::new(refused)).is_err(),
                "{refused}"
            );
        }
        assert!(check_root(&places, Path::new("/home/me/code/app")).is_ok());
    }
}
