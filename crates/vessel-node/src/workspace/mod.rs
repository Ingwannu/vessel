//! Projects, worktrees, git and files as the node serves them (protocol
//! 10, ADRs 0023 and 0027): who may call what, which folder a call works
//! in, and the state that outlives a call (projects, worktrees, approvals,
//! running clones and setups). The work itself is `vessel-workspace`'s.
//!
//! [Decision Log]
//! - Purpose: `vessel-workspace` knows git and files but no principal, no
//!   store and no session; the node decides who may call, keeps the record
//!   and ties a worktree to the sessions in it (ADR 0001's split, as
//!   `vessel-lsp` and `crate::lsp`).
//! - State: one registry in memory (projects with a `watch` of their
//!   revision for `project.get` to wait on, worktrees with a `watch` of
//!   their setup log's length for `worktree.output`, approvals), behind
//!   one lock held only to read or change it, never across an `.await`.
//!   Every change is written to the store whole (schema 9); the registry
//!   is read back when the node starts.
//! - A clone or a setup is a task of the node's tracker (it continues when
//!   its caller goes away, and stops with the node), with a token of its
//!   own so `project.remove` or `worktree.remove` can stop it.
//! - Roots that would widen what a caller can read are refused for every
//!   caller (`policy`): a file system's root, the user's home folder, and
//!   any folder holding the node's data folder (a read-only device could
//!   otherwise read the node's token and become its owner).

#[cfg(target_os = "linux")]
mod admission;
#[cfg(all(test, target_os = "linux"))]
mod checkpoints;
mod error;
#[cfg(any(test, target_os = "linux"))]
pub(crate) mod exclusion;
mod files;
mod forge;
mod git;
mod metadata;
mod policy;
mod projects;
mod roots;
mod scripts;
mod setup;
mod starting;
mod worktrees;
mod writing;

use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::transfers::Holder;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::workspace::{
    ApprovedCommands, CommandSet, CommandsDigest, GitSupport, ProjectInfo, WorktreeInfo,
};
use vessel_proto::{Creator, ProjectId, WorktreeId};
use vessel_store::{Store, StoreError};
use vessel_workspace::find::Indexes;
use vessel_workspace::{Git, GitSetup};

pub(crate) use error::WorkspaceError;
pub(crate) use git::IndexChange;
pub(crate) use policy::refuse_agents;
pub(crate) use projects::Occupant;
pub(crate) use scripts::{DirError, ScriptRoot};
pub(crate) use starting::StartGuard;
pub(crate) use worktrees::{Placed, Starting, TidyScope};

/// The most projects, and the most worktrees, one node keeps.
pub(crate) const MAX_PROJECTS: usize = 1000;
/// The most worktrees one node keeps.
pub(crate) const MAX_WORKTREES: usize = 1000;
/// The most worktrees an agent may have made and not removed.
pub(crate) const MAX_PER_AGENT: usize = 8;
/// The longest a long poll waits.
pub(crate) const MAX_WAIT_SECS: u32 = 30;

/// The folders the node keeps for worktrees.
#[derive(Debug, Clone, Default)]
struct Dirs {
    /// `<data>/workspace/worktrees`: one folder per project.
    worktrees: PathBuf,
    /// `<data>/workspace/logs`: each worktree's setup output.
    logs: PathBuf,
    /// `<data>/workspace/scratch`: temporary files.
    scratch: PathBuf,
    /// `<data>/workspace/no-hooks`: the empty hooks folder git is pointed
    /// at for every run that must not run the repository's hooks.
    no_hooks: PathBuf,
}

/// Where the workspace calls find what they need, as the node starts.
#[derive(Debug)]
pub(crate) struct Opening {
    /// The node's data folder.
    pub(crate) data: PathBuf,
    /// Its `workspace` folder, prepared.
    pub(crate) workspace: PathBuf,
    /// The user's home folder.
    pub(crate) home: PathBuf,
    /// The `git` program found on the node's search path.
    pub(crate) git: Option<PathBuf>,
    /// Added to git's environment: nothing for a node; the tests' own
    /// configuration, so a developer's settings never change a result.
    pub(crate) git_env: Vec<(String, OsString)>,
    /// Removed from the environment of setup and teardown (protocol 11):
    /// what a terminal's loses too (`TerminalsConfig::removed`), since
    /// both are a repository's commands the node starts.
    pub(crate) removed: Arc<[OsString]>,
}

/// Where the node runs and what roots it refuses.
#[derive(Debug, Clone, Default)]
pub(crate) struct Places {
    /// The node's data folder, resolved.
    pub(crate) data: PathBuf,
    /// The user's home folder, resolved.
    pub(crate) home: PathBuf,
}

/// A project as the node keeps it.
#[derive(Debug)]
struct ProjectEntry {
    info: ProjectInfo,
    /// The revision, for `project.get` to wait on.
    revision: watch::Sender<u64>,
    /// Stops its clone, while one runs.
    clone: Option<CancellationToken>,
}

/// A worktree as the node keeps it.
#[derive(Debug)]
struct WorktreeEntry {
    info: WorktreeInfo,
    /// The setup log's length, for `worktree.output` to wait on.
    written: Arc<watch::Sender<u64>>,
    /// Stops its setup or teardown, while one runs.
    running: Option<CancellationToken>,
    /// Sessions that passed `place` (or `session.resume`'s check) and are
    /// not live yet (ADR 0030): a removal counts them as sessions in it.
    starting: usize,
}

#[derive(Debug, Default)]
pub(super) struct State {
    projects: HashMap<ProjectId, ProjectEntry>,
    worktrees: HashMap<WorktreeId, WorktreeEntry>,
    approvals: HashSet<(ProjectId, CommandsDigest)>,
    /// What the newest approvals of each project covered, newest first,
    /// at most `vessel_store::MAX_APPROVED_SETS` (protocol 11): the first
    /// is shown beside a set that is not approved.
    approved_sets: HashMap<ProjectId, Vec<ApprovedCommands>>,
    /// The sets described lately, by digest, newest last, at most
    /// `projects::commands::DESCRIBED`: what an approval of a digest
    /// covers (the digest is of the set, so one found here is exactly it).
    described: VecDeque<(CommandsDigest, Arc<CommandSet>)>,
    /// Who is making a worktree now, one entry each, counted with the
    /// worktrees against the limits, so creations at once cannot all pass
    /// the check before any is recorded.
    making: Vec<Creator>,
}

/// The node's projects and worktrees.
#[derive(Debug)]
pub(crate) struct Workspaces {
    git: Option<Git>,
    root_reads: Arc<tokio::sync::Semaphore>,
    #[cfg(target_os = "linux")]
    exclusion: exclusion::RootGate,
    support: GitSupport,
    store: Store,
    dirs: Dirs,
    places: Places,
    state: Mutex<State>,
    indexes: Arc<Indexes>,
    /// Running `file.find`s by caller and key, so a newer one stops the
    /// older one.
    searches: Mutex<HashMap<(Holder, u32), (u64, CancellationToken)>>,
    /// Numbers each `file.find`, so one removes only its own entry.
    search_count: AtomicU64,
    /// Taken out of setup's and teardown's environment (`Opening`).
    removed: Arc<[OsString]>,
    tasks: TaskTracker,
    stop: CancellationToken,
}

impl Workspaces {
    /// Find git and read the record back. `workspace` is the data folder's
    /// `workspace` folder, prepared under the node's lock.
    ///
    /// # Errors
    /// The store cannot be read.
    pub(crate) async fn start(
        found: Opening,
        store: Store,
        tasks: TaskTracker,
        stop: CancellationToken,
    ) -> Result<Arc<Self>, StoreError> {
        let Opening {
            data,
            workspace,
            home,
            git,
            git_env,
            removed,
        } = found;
        let (places, dirs) = tokio::task::spawn_blocking(move || {
            // Absolute, always: git takes `no_hooks` as `core.hooksPath`,
            // which it reads relative to the repository it runs in, so a
            // node started with a relative data folder would otherwise run
            // the hooks a repository ships at that path.
            let resolve = |path: &Path| {
                std::fs::canonicalize(path)
                    .or_else(|_missing| std::path::absolute(path))
                    .unwrap_or_else(|_| path.to_path_buf())
            };
            let places = Places {
                data: resolve(&data),
                home: resolve(&home),
            };
            let dirs = Dirs {
                worktrees: resolve(&workspace.join("worktrees")),
                logs: resolve(&workspace.join("logs")),
                scratch: resolve(&workspace.join("scratch")),
                no_hooks: resolve(&workspace.join("no-hooks")),
            };
            (places, dirs)
        })
        .await
        .unwrap_or_else(|_panicked| (Places::default(), Dirs::default()));
        let found = Git::probe(git.map(|program| GitSetup {
            program,
            no_hooks: dirs.no_hooks.clone(),
            env: git_env,
        }))
        .await;
        tracing::info!(git = ?found.support, "workspace calls ready");
        let stored = store.workspace().await?;
        #[cfg(target_os = "linux")]
        let recovering = admission::recovering(&store).await?;
        #[cfg(target_os = "linux")]
        let (exclusion, owner) = exclusion::RootGate::start(recovering, stop.child_token());
        #[cfg(target_os = "linux")]
        let _owner = tasks.spawn(async move {
            if let Err(error) = owner.await {
                tracing::error!(%error, "physical root admission owner stopped unexpectedly");
            }
        });
        let workspaces = Arc::new(Self {
            git: found.git,
            root_reads: Arc::new(tokio::sync::Semaphore::new(metadata::ROOT_METADATA_WORKERS)),
            #[cfg(target_os = "linux")]
            exclusion,
            support: found.support,
            store,
            dirs,
            places,
            state: Mutex::new(State::default()),
            indexes: Arc::new(Indexes::default()),
            searches: Mutex::new(HashMap::new()),
            search_count: AtomicU64::new(0),
            removed,
            tasks,
            stop,
        });
        workspaces.restore(stored).await;
        Ok(workspaces)
    }

    /// The registry, locked: held only while it is read or changed.
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The node's git, or [`WorkspaceError::NoGit`].
    fn git(&self) -> Result<&Git, WorkspaceError> {
        self.git.as_ref().ok_or(WorkspaceError::NoGit)
    }

    /// Whether `path`, resolved (its nearest existing folder, when it is
    /// not there yet), is the node's data folder or inside it.
    pub(crate) async fn in_data_folder(&self, path: &Path) -> bool {
        let data = self.places.data.clone();
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let mut probe = path.as_path();
            loop {
                if let Ok(resolved) = std::fs::canonicalize(probe) {
                    return resolved.starts_with(&data);
                }
                match probe.parent() {
                    Some(parent) => probe = parent,
                    None => return false,
                }
            }
        })
        .await
        .unwrap_or(true)
    }

    /// Write a project's row.
    async fn save_project(&self, info: ProjectInfo) -> Result<(), WorkspaceError> {
        self.store.put_project(info).await.map_err(Into::into)
    }

    /// Write a worktree's row.
    async fn save_worktree(&self, info: WorktreeInfo) -> Result<(), WorkspaceError> {
        self.store.put_worktree(info).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests;
