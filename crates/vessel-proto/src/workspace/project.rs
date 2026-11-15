//! Projects: the folders a node works in (protocol 10, ADR 0027).

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::support::GitSupport;
use super::text::{CommandsDigest, FolderName, ProjectCommand, ProjectName, RefName};
use super::url::CloneUrl;
use super::worktree::WorktreeBase;
use crate::id::ProjectId;

/// A folder the node works in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProjectInfo {
    /// Its id.
    pub id: ProjectId,
    /// What it is called (its folder's name unless given one).
    pub name: ProjectName,
    /// The folder on the node, absolute: a repository's top level when the
    /// folder added was inside one.
    pub root: PathBuf,
    /// Whether it is a git repository. A folder that is not has files but
    /// no git and no worktrees.
    pub git: bool,
    /// Where new worktrees start: the remote-tracking branch `origin/HEAD`
    /// names (`origin/main`), else a local `main`, else `master`, else the
    /// branch the folder has checked out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<RefName>,
    /// Ready, or being cloned.
    pub state: ProjectState,
    /// When it was added.
    pub created_at: DateTime<Utc>,
    /// Grows by one at every change (a clone's progress, its end), so a
    /// client can wait for the next (`project.get` with `after`).
    pub revision: u64,
}

/// Whether a project can be used yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ProjectState {
    /// Usable.
    Ready,
    /// Being cloned.
    Cloning {
        /// What git is doing.
        stage: CloneStage,
        /// How far along that stage is, when git says.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        percent: Option<u8>,
    },
    /// The clone failed; nothing was left behind. Remove the project to
    /// forget it, or clone again.
    CloneFailed {
        /// Why, for a client to act on.
        reason: CloneFailure,
        /// git's last lines, credentials removed, for a person.
        message: String,
    },
    /// Its folder is gone.
    Missing,
    /// A state a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// A clone's stage, from git's progress lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CloneStage {
    /// Reaching the remote.
    Connecting,
    /// "Counting objects" and "Enumerating objects".
    Counting,
    /// "Compressing objects" on the remote.
    Compressing,
    /// "Receiving objects".
    Receiving,
    /// "Resolving deltas".
    Resolving,
    /// "Updating files": writing the working tree.
    CheckingOut,
    /// A stage a newer node knows.
    #[serde(other)]
    Unknown,
}

/// Why a clone failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CloneFailure {
    /// The remote wants a credential git has none for (prompts are off).
    NeedsCredential,
    /// No such repository, or no access to it.
    NotFound,
    /// The host could not be reached.
    Network,
    /// It ran past the clone's time limit.
    TimedOut,
    /// Anything else (see the message).
    Other,
    /// A reason a newer node knows.
    #[serde(other)]
    Unknown,
}

/// The projects, and what git the node found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProjectList {
    /// Whether the git calls can work on this node.
    pub git: GitSupport,
    /// Every project, oldest first, at most 1,000.
    pub projects: Vec<ProjectInfo>,
    /// Whether there were more.
    pub more: bool,
}

/// The commands a project's file (`vessel.toml`) names, and whether a
/// person approved that set (ADR 0027).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ProjectCommands {
    /// No file, or no data that requires project approval.
    None,
    /// The commands, in order.
    Listed {
        /// Run when a worktree is created.
        setup: Vec<ProjectCommand>,
        /// Run before a worktree is removed.
        teardown: Vec<ProjectCommand>,
        /// What approving this set approves.
        digest: CommandsDigest,
        /// Whether a person approved it for this project.
        approved: bool,
        /// Its scripts and services, in name order (protocol 11, ADR
        /// 0031); the digest covers them. Left out when there are none.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        scripts: Vec<crate::scripts::ScriptEntry>,
        /// Terminal commands the client must show before approval (ADR 0066).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        terminals: Vec<super::FileTerminal>,
        /// Allocation choices that must be displayed or refused before approval.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        service_ports: Option<super::ServicePorts>,
        /// The set last approved for this project, to show what changed
        /// (protocol 11): only while this set is not approved, and only
        /// when the node kept what that approval covered (approvals given
        /// from protocol 11 on). Bounded as this set is.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_approved: Option<Box<super::ApprovedCommands>>,
    },
    /// The file does not parse; nothing in it runs.
    Invalid {
        /// Why.
        message: String,
    },
}

/// `project.list`: every project.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListProjects {}

/// `project.get`: one project, at once or after its next change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetProject {
    /// Which.
    pub project: ProjectId,
    /// Wait (at most 30 s) until its `revision` differs from this one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_from: Option<u64>,
}

/// `project.add`: a folder on the node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AddProject {
    /// An existing folder, absolute.
    pub path: PathBuf,
    /// What to call it; its folder's name otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<ProjectName>,
}

/// `project.clone`: a repository into a new folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CloneProject {
    /// What to clone.
    pub url: CloneUrl,
    /// The existing folder, absolute, to clone into.
    pub parent: PathBuf,
    /// The new folder in it; the repository's name otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<FolderName>,
    /// What to call the project; the folder's name otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<ProjectName>,
}

/// `project.init`: a new repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InitProject {
    /// The folder, absolute: absent or empty.
    pub path: PathBuf,
    /// What to call it; its folder's name otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<ProjectName>,
}

/// `project.remove`: forget a project (its files stay).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RemoveProject {
    /// Which.
    pub project: ProjectId,
}

/// `project.commands`: what the project's file would run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetProjectCommands {
    /// Which project.
    pub project: ProjectId,
    /// The file at this commit or ref; the project's folder as it is
    /// otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<WorktreeBase>,
}

/// `project.approve`: approve (or withdraw) a set of commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ApproveCommands {
    /// Which project.
    pub project: ProjectId,
    /// The set, as `project.commands` gave it.
    pub digest: CommandsDigest,
    /// Approve, or withdraw an approval.
    pub approved: bool,
}
