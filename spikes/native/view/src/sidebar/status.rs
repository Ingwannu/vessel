//! A sidebar row's status in Paseo's buckets (TRACKER 662/663,
//! `utils/sidebar-agent-state.ts`): needs input, failed, running,
//! attention (finished and not seen), done; and a project's aggregate, the
//! most urgent of its sessions (`getProjectStatusBadgeContent`'s input).
//!
//! [Decision Log]
//! - Purpose: a session's row and its project's badge say the same status
//!   in Paseo's words and marks.
//! - Existing: `attention::Bucket` is Paseo's five in its urgency order,
//!   with its fold (`attention::workspace`); the sidebar's status groups
//!   keep the page's four marks (`SessionMark`), unchanged here.
//! - Choice: a session's bucket from what its card knows: it waits on its
//!   owner (the board's needs-you, which also counts failing checks and
//!   requested changes: Vessel's own reasons beyond Paseo's pending
//!   permission), its process failed, its agent works or starts, it has
//!   unread work, else done; a project's is the fold over its listed
//!   sessions on that machine (archived never, ended ones when shown).
//! - Cost: a project's sessions are those whose node names the project
//!   (`in_project`, protocol 10); a session started outside any project
//!   counts for none.

use vessel_proto::ProjectId;

use super::listed;
use crate::attention::{AgentContribution, Bucket, workspace};
use crate::board::{Card, Fleet, Phrase};
use crate::prefs::SidebarView;

/// What a session runs in, for the glyph a failed row shows (Paseo's
/// `workspaceKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceKind {
    /// A project's own folder (Paseo's `local_checkout`, a `Monitor`).
    Checkout,
    /// One of a project's worktrees (a `FolderGit2`).
    Worktree,
    /// A folder outside any project (a `Folder`).
    Folder,
}

impl Card {
    /// Its row's bucket.
    #[must_use]
    pub fn bucket(&self) -> Bucket {
        if self.needs_you() {
            Bucket::NeedsInput
        } else if self.detail.failure.is_some() {
            Bucket::Failed
        } else if self.is_working() || (self.live && self.detail.process == Phrase::Starting) {
            // Starting and working are both busy: one ring (Paseo's
            // `loading` resolves to running).
            Bucket::Working
        } else if self.detail.marks.unread {
            Bucket::Review
        } else {
            Bucket::Done
        }
    }

    /// What it runs in.
    #[must_use]
    pub const fn workspace_kind(&self) -> WorkspaceKind {
        match (self.detail.in_project, self.detail.in_worktree) {
            (Some(_), true) => WorkspaceKind::Worktree,
            (Some(_), false) => WorkspaceKind::Checkout,
            (None, _) => WorkspaceKind::Folder,
        }
    }
}

impl Fleet {
    /// `project`'s status on `machine`: the most urgent bucket of its
    /// sessions the sidebar lists (`view`); done when it has none; `None`
    /// for a machine not in the book.
    #[must_use]
    pub fn project_bucket(
        &self,
        machine: &str,
        project: ProjectId,
        view: SidebarView,
    ) -> Option<Bucket> {
        let machine = self.machines.iter().find(|each| each.name == machine)?;
        let sessions = listed(&machine.sessions, view)
            .into_iter()
            .filter(|card| card.detail.in_project == Some(project))
            .map(|card| AgentContribution::Root(card.bucket()));
        workspace(sessions, [])
    }
}
