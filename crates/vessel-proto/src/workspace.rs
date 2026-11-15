//! Projects, worktrees, git and files (protocol 10, milestone 11 track B;
//! ADRs 0023 and 0027): the wire types of the calls a node answers about
//! the folders it works in.
//!
//! One module per area. Every string with a shape is a parsed newtype
//! ([`text`], [`path`], [`url`]), so a request that decodes names only
//! paths that stay inside their root and git names that cannot read as
//! options.

mod approval;
mod diff;
mod exec_path;
mod file_terminals;
mod files;
mod git;
mod path;
mod port_range;
mod port_script;
mod project;
mod project_text;
mod service_port;
mod service_ports;
mod support;
mod text;
mod url;
mod worktree;

pub use approval::{ApprovedCommands, CommandSet};
pub use diff::{
    ChangeStatus, DiffContent, DiffLine, DiffScope, DiffSummary, FileChange, FileDiff, GetChanges,
    GetFileDiff, Hunk, LineKind, ModeChange,
};
pub use exec_path::{ExecPath, ExecPathSyntax, InvalidExecPath};
pub use file_terminals::{FileTerminal, PROJECT_FILE_KEYS};
pub use files::{
    CreateDir, DeletePath, DirEntry, DirListing, EntryKind, FileContent, FileRead, FileVersion,
    FindFiles, FoundFile, FoundFiles, GrepCase, GrepMatch, GrepMode, GrepResult, ListDir, ReadFile,
    RenamePath, SearchFiles, WriteExpect, WriteFile,
};
pub use git::{
    Branch, BranchList, Commit, CommitInfo, CommitOutcome, CommitPage, CommitStage, Discard,
    Distance, Fetch, FetchOutcome, FileState, GetStatus, GitRefusal, GitStatus, ListBranches,
    ListStashes, PathSet, Pull, PullOutcome, Push, PushOutcome, ReadLog, RenameBranch,
    RenameOutcome, RepositoryConfigReason, ShowFile, Stage, StashAction, StashChanges, StashEntry,
    StashList, StashOutcome, StatusEntry, Switch, SwitchOutcome, Unstage, WorkspaceRoot,
};
pub use path::{InvalidRelPath, RelPath, is_git_dir};
pub use port_range::{InvalidPortRange, PortRange};
pub use port_script::{InvalidPortScript, PortScript, PortScriptPath};
pub use project::{
    AddProject, ApproveCommands, CloneFailure, CloneProject, CloneStage, GetProject,
    GetProjectCommands, InitProject, ListProjects, ProjectCommands, ProjectInfo, ProjectList,
    ProjectState, RemoveProject,
};
pub use project_text::trim_project_text;
pub use service_port::{InvalidServicePort, ServicePort};
pub use service_ports::{InvalidServicePorts, ServicePorts, ServicePortsView};
pub use support::{GitSupport, GitVersion};
pub use text::{
    BranchName, CommandsDigest, CommitId, CommitMessage, ConfigKey, ContentHash, FindQuery,
    FolderName, GrepQuery, InvalidText, ProjectCommand, ProjectName, RefName, RemoteName,
    SearchKey, TextKind, TextProblem,
};
pub use url::{CloneUrl, InvalidCloneUrl};
pub use worktree::{
    CreateWorktree, CreateWorktreeOutcome, DropSnapshot, GitOperation, IfLost, KeptReason,
    ListSnapshots, ListWorktrees, ReadSetupOutput, RemoveWorktree, RemoveWorktreeOutcome, RunSetup,
    SessionPlace, SessionWorkspace, SetupChoice, SetupFailure, SetupOutput, SetupState, Snapshot,
    SnapshotList, WorktreeBase, WorktreeBranch, WorktreeInfo, WorktreeList, WorktreeRefusal,
    WorktreeStart,
};
