//! What the node answered a workspace ask, with what was asked, and the
//! answer an ask gets when it could not be sent at all.

use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::save::{SaveFailure, Saved};
use vessel_proto::workspace::{
    BranchList, CommandsDigest, CommitOutcome, CommitPage, DiffSummary, DirListing, DropSnapshot,
    FetchOutcome, FileDiff, FileRead, FindFiles, FoundFiles, GetChanges, GetFileDiff, GitStatus,
    GrepResult, ListDir, ProjectCommands, ProjectInfo, ProjectList, PullOutcome, PushOutcome,
    ReadFile, ReadLog, RelPath, RemoveWorktree, RemoveWorktreeOutcome, RenameOutcome, SearchFiles,
    SnapshotList, StashList, StashOutcome, SwitchOutcome, WorkspaceRoot, WorktreeInfo,
    WorktreeList,
};
use vessel_proto::{ProjectId, SessionId, SessionInfo, WorktreeId};

use super::{
    Ask, FileAsk, GitAsk, GitWrite, PathChange, ProjectAsk, StartEnded, StartKey, WorktreeAsk,
};

/// What a git write answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitWritten {
    /// Stage, unstage, discard: the status after.
    Status(GitStatus),
    /// A commit, or its refusal.
    Commit(CommitOutcome),
    /// A fetch, or its refusal.
    Fetch(FetchOutcome),
    /// A push, or its refusal.
    Push(PushOutcome),
    /// A pull, or its refusal.
    Pull(PullOutcome),
    /// A switch, or its refusal.
    Switch(SwitchOutcome),
    /// A rename, or its refusal.
    Rename(RenameOutcome),
    /// A stash action, or its refusal.
    Stash(StashOutcome),
}

/// What the node answered a workspace ask, with what was asked.
#[derive(Debug)]
pub enum Answer {
    /// The machine's projects.
    Projects(Result<ProjectList, Problem>),
    /// A project as it is now (a clone's progress, or its end).
    Project(ProjectInfo),
    /// A project added, cloned or started (or why not).
    Took {
        /// Which form asked.
        form: Form,
        /// The project.
        result: Result<ProjectInfo, Problem>,
    },
    /// A project forgotten (or why not).
    Forgot {
        /// Which.
        project: ProjectId,
        /// Done, or why not.
        result: Result<(), Problem>,
    },
    /// A project's commands, read or approved.
    Commands {
        /// Which project.
        project: ProjectId,
        /// The set asked for, when approving.
        approved: Option<(CommandsDigest, bool)>,
        /// The commands.
        result: Result<ProjectCommands, Problem>,
    },
    /// Worktrees.
    Worktrees {
        /// The project asked for, if one.
        project: Option<ProjectId>,
        /// The list.
        result: Result<WorktreeList, Problem>,
    },
    /// A project's snapshots.
    Snapshots {
        /// Which project.
        project: ProjectId,
        /// The list.
        result: Result<SnapshotList, Problem>,
    },
    /// A snapshot dropped.
    Dropped {
        /// Which.
        ask: DropSnapshot,
        /// Done, or why not.
        result: Result<(), Problem>,
    },
    /// Setup started again.
    SetupStarted {
        /// Which worktree.
        worktree: WorktreeId,
        /// It, as it is now.
        result: Result<WorktreeInfo, Problem>,
    },
    /// A worktree removed, or kept with its reasons.
    Removed {
        /// What was asked.
        ask: RemoveWorktree,
        /// The outcome.
        result: Result<RemoveWorktreeOutcome, Problem>,
    },
    /// A root's status.
    Status {
        /// Which root.
        root: WorkspaceRoot,
        /// It.
        result: Result<GitStatus, Problem>,
    },
    /// The files a comparison changes.
    Changes {
        /// What was asked.
        ask: GetChanges,
        /// The summary.
        result: Result<DiffSummary, Problem>,
    },
    /// A page of a file's diff.
    Diff {
        /// What was asked.
        ask: GetFileDiff,
        /// The page.
        result: Result<FileDiff, Problem>,
    },
    /// Branches.
    Branches {
        /// Which root.
        root: WorkspaceRoot,
        /// The list.
        result: Result<BranchList, Problem>,
    },
    /// A page of the log.
    Log {
        /// What was asked.
        ask: ReadLog,
        /// The page.
        result: Result<CommitPage, Problem>,
    },
    /// The stash.
    Stashes {
        /// Which root.
        root: WorkspaceRoot,
        /// The list.
        result: Result<StashList, Problem>,
    },
    /// A git write's answer.
    Git {
        /// What was asked.
        write: GitWrite,
        /// The outcome, a refusal included.
        result: Result<GitWritten, Problem>,
    },
    /// A folder's entries.
    Listed {
        /// What was asked.
        ask: ListDir,
        /// The entries.
        result: Result<DirListing, Problem>,
    },
    /// A page of a file.
    Read {
        /// What was asked.
        ask: ReadFile,
        /// The page.
        result: Result<FileRead, Problem>,
    },
    /// A save's outcome.
    Saved {
        /// The root.
        root: WorkspaceRoot,
        /// The file.
        path: RelPath,
        /// What it came to.
        saved: Saved,
    },
    /// Files found by name.
    Found {
        /// What was asked.
        ask: FindFiles,
        /// The matches.
        result: Result<FoundFiles, Problem>,
    },
    /// Lines found in files.
    Grepped {
        /// What was asked.
        ask: SearchFiles,
        /// The matches.
        result: Result<GrepResult, Problem>,
    },
    /// A path made, renamed or deleted.
    PathChanged {
        /// What was asked.
        change: PathChange,
        /// Done, or why not.
        result: Result<(), Problem>,
    },
    /// A step of a start.
    Start {
        /// Which start.
        key: StartKey,
        /// The step.
        step: vessel_app_view::workspace::start::Step,
    },
    /// A start's end.
    StartEnded {
        /// Which start.
        key: StartKey,
        /// How it ended.
        ended: StartEnded,
    },
    /// Where a session runs.
    Session {
        /// Which.
        session: SessionId,
        /// It (boxed: a session's info is large beside the other answers).
        result: Result<Box<SessionInfo>, Problem>,
    },
    /// A session's pull request.
    Pull(super::PullAnswer),
}

/// Which form added a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// Add a folder.
    Folder,
    /// Clone a repository.
    Clone,
    /// Start a new project.
    New,
}

impl Ask {
    /// The answer this ask gets when it did not reach the machine
    /// (`problem`: why), as each screen expects it: its key in it, so
    /// what waits on that key settles (the app's ask the link did not
    /// take, `Problem::Unsent`).
    #[must_use]
    pub fn refused(self, problem: &Problem) -> Answer {
        refused(self, problem)
    }
}

/// The answer an ask gets when it could not be sent at all.
pub(super) fn refused(ask: Ask, problem: &Problem) -> Answer {
    match ask {
        Ask::Project(ask) => project_refused(ask, problem),
        Ask::Worktree(ask) => worktree_refused(ask, problem),
        Ask::Git(ask) => git_refused(ask, problem),
        Ask::File(ask) => file_refused(ask, problem),
        Ask::Start(start) => Answer::StartEnded {
            key: start.key,
            ended: StartEnded::Failed(problem.clone()),
        },
        Ask::Session(session) => Answer::Session {
            session,
            result: no(problem),
        },
        Ask::Pull(ask) => Answer::Pull(super::pulls::refused(ask, problem)),
    }
}

fn project_refused(ask: ProjectAsk, problem: &Problem) -> Answer {
    match ask {
        ProjectAsk::List => Answer::Projects(no(problem)),
        ProjectAsk::Add(_) => took_failed(Form::Folder, problem),
        ProjectAsk::Clone(_) => took_failed(Form::Clone, problem),
        ProjectAsk::Init(_) => took_failed(Form::New, problem),
        ProjectAsk::Remove(project) => Answer::Forgot {
            project,
            result: no(problem),
        },
        ProjectAsk::Commands(ask) => Answer::Commands {
            project: ask.project,
            approved: None,
            result: no(problem),
        },
        ProjectAsk::Approve(ask) => Answer::Commands {
            project: ask.project,
            approved: Some((ask.digest, ask.approved)),
            result: no(problem),
        },
    }
}

fn worktree_refused(ask: WorktreeAsk, problem: &Problem) -> Answer {
    match ask {
        WorktreeAsk::List(ask) => Answer::Worktrees {
            project: ask.project,
            result: no(problem),
        },
        WorktreeAsk::Snapshots(project) => Answer::Snapshots {
            project,
            result: no(problem),
        },
        WorktreeAsk::DropSnapshot(ask) => Answer::Dropped {
            ask,
            result: no(problem),
        },
        WorktreeAsk::RunSetup(worktree) => Answer::SetupStarted {
            worktree,
            result: no(problem),
        },
        WorktreeAsk::Remove(ask) => Answer::Removed {
            ask,
            result: no(problem),
        },
    }
}

fn git_refused(ask: GitAsk, problem: &Problem) -> Answer {
    match ask {
        GitAsk::Status(root) => Answer::Status {
            root,
            result: no(problem),
        },
        GitAsk::Changes(ask) => Answer::Changes {
            ask,
            result: no(problem),
        },
        GitAsk::Diff(ask) => Answer::Diff {
            ask,
            result: no(problem),
        },
        GitAsk::Branches(root) => Answer::Branches {
            root,
            result: no(problem),
        },
        GitAsk::Log(ask) => Answer::Log {
            ask,
            result: no(problem),
        },
        GitAsk::Stashes(root) => Answer::Stashes {
            root,
            result: no(problem),
        },
        GitAsk::Write(write) => Answer::Git {
            write,
            result: no(problem),
        },
    }
}

fn file_refused(ask: FileAsk, problem: &Problem) -> Answer {
    match ask {
        FileAsk::List(ask) => Answer::Listed {
            ask,
            result: no(problem),
        },
        FileAsk::Read(ask) => Answer::Read {
            ask,
            result: no(problem),
        },
        FileAsk::Save(save) => Answer::Saved {
            root: save.root,
            path: save.path,
            saved: Saved::Failed(SaveFailure::Call(problem.clone())),
        },
        FileAsk::Find(ask) => Answer::Found {
            ask,
            result: no(problem),
        },
        FileAsk::Grep(ask) => Answer::Grepped {
            ask,
            result: no(problem),
        },
        FileAsk::Path(change) => Answer::PathChanged {
            change,
            result: no(problem),
        },
    }
}

/// An error result of any type, for [`refused`].
pub(super) fn no<T>(problem: &Problem) -> Result<T, Problem> {
    Err(problem.clone())
}

fn took_failed(form: Form, problem: &Problem) -> Answer {
    Answer::Took {
        form,
        result: no(problem),
    }
}
