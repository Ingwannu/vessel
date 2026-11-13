//! Which project or worktree a session's code panel reads: the session's
//! own worktree when it has one, else its project's folder. A session
//! started in a typed folder has neither, and its panel offers to add the
//! folder as a project instead (the page's `data/workspace/root.ts`).

use vessel_proto::SessionInfo;
use vessel_proto::workspace::{BranchName, WorkspaceRoot};

/// The root a session's panel reads, or `None` when it is in no project.
#[must_use]
pub fn session_root(session: &SessionInfo) -> Option<WorkspaceRoot> {
    let workspace = session.workspace.as_deref()?;
    Some(workspace.worktree.map_or_else(
        || WorkspaceRoot::Project {
            project: workspace.project,
        },
        |worktree| WorkspaceRoot::Worktree { worktree },
    ))
}

/// The session's branch, when it is in a repository on one.
#[must_use]
pub fn session_branch(session: &SessionInfo) -> Option<&BranchName> {
    session.workspace.as_deref()?.branch.as_ref()
}
