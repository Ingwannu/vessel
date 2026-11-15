//! What a workspace screen asks: one enum per subject, each variant
//! holding the wire's own request.

use vessel_proto::workspace::{
    AddProject, ApproveCommands, CloneProject, CreateDir, DeletePath, Discard, DropSnapshot, Fetch,
    FileVersion, FindFiles, GetChanges, GetFileDiff, GetProjectCommands, InitProject, ListDir,
    ListWorktrees, Pull, Push, ReadFile, ReadLog, RelPath, RemoveWorktree, RenameBranch,
    RenamePath, SearchFiles, Stage, StashChanges, Switch, Unstage, WorkspaceRoot,
};
use vessel_proto::{ProjectId, SessionId, WorktreeId};

use super::StartAsk;

/// What a workspace screen asks of a machine.
#[derive(Debug)]
pub enum Ask {
    /// Complete command-set reads and grants on the original admission.
    CommandAccess(super::command_access::Ask),
    /// A project's supervised scripts and discovered ports.
    Scripts(super::ScriptAsk),
    /// Correlated backing-entity actions from workspace tabs.
    Layout(super::layout::Ask),
    /// About its projects.
    Project(ProjectAsk),
    /// About worktrees and snapshots.
    Worktree(WorktreeAsk),
    /// About a root's git.
    Git(GitAsk),
    /// About a root's files.
    File(FileAsk),
    /// A session started in a new worktree, walked to its end.
    Start(Box<StartAsk>),
    /// `session.get`: where a session runs (the code panel's root).
    Session(SessionId),
    /// A session's pull request (`pr.get`, `pr.open`).
    Pull(super::PullAsk),
}

/// The project asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectAsk {
    /// `project.list`; then each project being cloned is followed.
    List,
    /// `project.add`.
    Add(AddProject),
    /// `project.clone`; followed until the clone ends.
    Clone(CloneProject),
    /// `project.init`.
    Init(InitProject),
    /// `project.remove`.
    Remove(ProjectId),
    /// `project.commands`.
    Commands(GetProjectCommands),
    /// `project.approve` (or withdraw).
    Approve(ApproveCommands),
}

/// The worktree asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeAsk {
    /// `worktree.list`.
    List(ListWorktrees),
    /// `worktree.snapshots`.
    Snapshots(ProjectId),
    /// `worktree.drop_snapshot`.
    DropSnapshot(DropSnapshot),
    /// `worktree.setup`.
    RunSetup(WorktreeId),
    /// `worktree.remove`.
    Remove(RemoveWorktree),
}

/// The git asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitAsk {
    /// `git.status`.
    Status(WorkspaceRoot),
    /// `git.changes`.
    Changes(GetChanges),
    /// `git.diff`, a page of one file's hunks.
    Diff(GetFileDiff),
    /// `git.branches`.
    Branches(WorkspaceRoot),
    /// `git.log`.
    Log(ReadLog),
    /// `git.stashes`.
    Stashes(WorkspaceRoot),
    /// A git write.
    Write(GitWrite),
}

/// The file asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAsk {
    /// `file.list`.
    List(ListDir),
    /// `file.read`.
    Read(ReadFile),
    /// A guarded save (`file.write`, then `file.read` once on a conflict).
    Save(Save),
    /// `file.find`.
    Find(FindFiles),
    /// `file.grep`.
    Grep(SearchFiles),
    /// `file.create_dir`, `file.rename`, `file.delete`, or a new file.
    Path(PathChange),
}

/// A git write (`git.stage` to `git.stash`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitWrite {
    /// Stage paths.
    Stage(Stage),
    /// Unstage paths.
    Unstage(Unstage),
    /// Discard changes.
    Discard(Discard),
    /// Commit.
    Commit(vessel_proto::workspace::Commit),
    /// Fetch.
    Fetch(Fetch),
    /// Push.
    Push(Push),
    /// Pull, fast-forward only.
    Pull(Pull),
    /// Switch or create a branch.
    Switch(Switch),
    /// Rename the current branch.
    RenameBranch(RenameBranch),
    /// Save, apply, pop or drop a stash entry.
    Stash(StashChanges),
}

impl GitWrite {
    /// The root it changes.
    #[must_use]
    pub const fn root(&self) -> &WorkspaceRoot {
        match self {
            Self::Stage(Stage { root, .. })
            | Self::Unstage(Unstage { root, .. })
            | Self::Discard(Discard { root, .. })
            | Self::Commit(vessel_proto::workspace::Commit { root, .. })
            | Self::Fetch(Fetch { root, .. })
            | Self::Push(Push { root, .. })
            | Self::Pull(Pull { root })
            | Self::Switch(Switch { root, .. })
            | Self::RenameBranch(RenameBranch { root, .. })
            | Self::Stash(StashChanges { root, .. }) => root,
        }
    }
}

/// A save of an edited file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Save {
    /// The root.
    pub root: WorkspaceRoot,
    /// The file.
    pub path: RelPath,
    /// The text to write.
    pub text: String,
    /// The version the edit started from; `None` is the person's
    /// Overwrite.
    pub base: Option<FileVersion>,
}

/// A path made, renamed or deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathChange {
    /// Make a folder.
    CreateDir(CreateDir),
    /// Rename or move, never over an existing path.
    Rename(RenamePath),
    /// Delete.
    Delete(DeletePath),
    /// Create an empty file (a write that expects nothing there).
    CreateFile {
        /// The root.
        root: WorkspaceRoot,
        /// The new file.
        path: RelPath,
    },
}
