//! [`Terminals`]: the node's terminals by id, their limits, and what ties
//! them to sessions, projects and agents.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use vessel_proto::scripts::ScriptName;
use vessel_proto::terminals::TerminalInfo;
use vessel_proto::{Creator, ErrorCode, NodeId, ProjectId, SessionId, TerminalId, WireError};
use vessel_session::terminals::TerminalHandle;

use super::scripts::Plan;
use super::{MAX_ENDED, MAX_LIVE, MAX_PER_AGENT, MAX_PER_PROJECT, MAX_PER_SESSION, Runner};
use crate::workspace::Occupant;

/// What a terminal counts against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Owner {
    /// The session it is beside.
    pub(crate) session: Option<SessionId>,
    /// The project it is placed in.
    pub(crate) project: Option<ProjectId>,
    /// The agent that opened it.
    pub(crate) agent: Option<(NodeId, SessionId)>,
}

impl Owner {
    fn of(info: &TerminalInfo) -> Self {
        Self {
            session: info.place.session(),
            project: info.place.project(),
            agent: match info.created_by {
                Creator::Agent { node, session } => Some((node, session)),
                Creator::Owner | Creator::Device { .. } => None,
            },
        }
    }
}

/// A terminal as the node keeps it.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) handle: TerminalHandle,
    /// What a script's run ran, to run it again.
    pub(crate) plan: Option<Arc<Plan>>,
}

#[derive(Debug, Default)]
struct State {
    entries: HashMap<TerminalId, Entry>,
    /// Starts in progress, counted with the live ones.
    starting: Vec<(u64, Owner)>,
    /// Numbers each reservation.
    counter: u64,
}

/// Why a terminal call failed, before it reaches the wire.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TerminalsError {
    /// No such terminal.
    #[error("no such terminal: {0}")]
    NotFound(TerminalId),
    /// A limit of the node, a session or a project.
    #[error("too many terminals: {0}")]
    Limit(&'static str),
    /// An agent's own limit, or what agents may not do.
    #[error("{0}")]
    Forbidden(String),
    /// It ended, or is not what the call needs.
    #[error("{0}")]
    Conflict(String),
    /// What the node or the terminal cannot do.
    #[error("{0}")]
    Unsupported(String),
    /// A request the node refuses as written.
    #[error("{0}")]
    Invalid(String),
    /// The shell did not start.
    #[error("the shell could not be started: {0}")]
    Spawn(String),
    /// The node is stopping.
    #[error("the node is stopping")]
    ShuttingDown,
    /// A failure already on the wire's terms (a place's, a session's).
    #[error("{}", .0.message)]
    Wire(WireError),
}

impl TerminalsError {
    /// The error a client sees.
    pub(crate) fn wire(self) -> WireError {
        let code = match &self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Limit(_) | Self::ShuttingDown => ErrorCode::Unavailable,
            Self::Forbidden(_) => ErrorCode::Forbidden,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::Unsupported(_) => ErrorCode::Unsupported,
            Self::Invalid(_) => ErrorCode::InvalidRequest,
            Self::Spawn(_) => ErrorCode::SpawnFailed,
            Self::Wire(error) => return error.clone(),
        };
        WireError::new(code, self.to_string())
    }
}

/// The node's terminals.
#[derive(Debug)]
pub(crate) struct Terminals {
    pub(crate) runner: Runner,
    state: Arc<Mutex<State>>,
}

/// A start counted against the limits until it is registered or dropped.
#[derive(Debug)]
pub(crate) struct Reservation {
    state: Arc<Mutex<State>>,
    number: u64,
}

impl Drop for Reservation {
    fn drop(&mut self) {
        lock(&self.state)
            .starting
            .retain(|(number, _)| *number != self.number);
    }
}

fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Terminals {
    pub(crate) fn new(runner: Runner) -> Arc<Self> {
        Arc::new(Self {
            runner,
            state: Arc::default(),
        })
    }

    fn state(&self) -> MutexGuard<'_, State> {
        lock(&self.state)
    }

    /// Count a start against the limits, or say which is full.
    ///
    /// # Errors
    /// `Limit` (the node's, a session's, a project's) or `Forbidden` (an
    /// agent's four).
    pub(crate) fn reserve(&self, owner: Owner) -> Result<Reservation, TerminalsError> {
        let mut state = self.state();
        let live: Vec<Owner> = state
            .entries
            .values()
            .filter(|entry| !entry.handle.is_ended())
            .map(|entry| Owner::of(&entry.handle.info()))
            .chain(state.starting.iter().map(|(_, owner)| owner.clone()))
            .collect();
        let count = |same: &dyn Fn(&Owner) -> bool| live.iter().filter(|other| same(other)).count();
        if live.len() >= MAX_LIVE {
            return Err(TerminalsError::Limit("at most 64 live terminals a node"));
        }
        if let Some(session) = owner.session
            && count(&|other| other.session == Some(session)) >= MAX_PER_SESSION
        {
            return Err(TerminalsError::Limit(
                "at most 8 live terminals beside a session",
            ));
        }
        if let Some(project) = owner.project
            && count(&|other| other.project == Some(project)) >= MAX_PER_PROJECT
        {
            return Err(TerminalsError::Limit(
                "at most 16 live terminals in a project",
            ));
        }
        if let Some(agent) = owner.agent
            && count(&|other| other.agent == Some(agent)) >= MAX_PER_AGENT
        {
            return Err(TerminalsError::Forbidden(
                "an agent may have at most 4 live terminals".to_owned(),
            ));
        }
        state.counter = state.counter.wrapping_add(1);
        let number = state.counter;
        state.starting.push((number, owner));
        Ok(Reservation {
            state: Arc::clone(&self.state),
            number,
        })
    }

    /// Keep a started terminal (its reservation ends), forgetting the
    /// oldest ended ones past the bound.
    pub(crate) fn register(&self, reservation: Reservation, entry: Entry) {
        let id = entry.handle.id();
        let mut state = self.state();
        drop(state.entries.insert(id, entry));
        let mut ended: Vec<(TerminalId, chrono::DateTime<chrono::Utc>)> = state
            .entries
            .values()
            .filter(|entry| entry.handle.is_ended())
            .map(|entry| {
                let info = entry.handle.info();
                (info.id, info.ended_at.unwrap_or(info.created_at))
            })
            .collect();
        if ended.len() > MAX_ENDED {
            ended.sort_by_key(|(_, at)| *at);
            let excess = ended.len().saturating_sub(MAX_ENDED);
            for (old, _) in ended.into_iter().take(excess) {
                drop(state.entries.remove(&old));
            }
        }
        drop(state);
        drop(reservation);
    }

    /// One terminal.
    pub(crate) fn get(&self, id: TerminalId) -> Result<Entry, TerminalsError> {
        self.state()
            .entries
            .get(&id)
            .cloned()
            .ok_or(TerminalsError::NotFound(id))
    }

    /// Forget one (it has ended, or is being closed).
    pub(crate) fn forget(&self, id: TerminalId) {
        drop(self.state().entries.remove(&id));
    }

    /// Every terminal `keep` lets through: live ones first, then ended
    /// ones, each newest first.
    pub(crate) fn list(&self, keep: impl Fn(&TerminalInfo) -> bool) -> Vec<TerminalInfo> {
        let mut infos: Vec<TerminalInfo> = self
            .state()
            .entries
            .values()
            .map(|entry| entry.handle.info())
            .filter(|info| keep(info))
            .collect();
        infos.sort_by(|a, b| {
            b.is_live()
                .cmp(&a.is_live())
                .then_with(|| b.created_at.cmp(&a.created_at))
        });
        infos
    }

    /// The handles `select` picks.
    pub(crate) fn handles(&self, select: impl Fn(&TerminalInfo) -> bool) -> Vec<TerminalHandle> {
        self.state()
            .entries
            .values()
            .filter(|entry| select(&entry.handle.info()))
            .map(|entry| entry.handle.clone())
            .collect()
    }

    /// The live terminals and the folders they run in: each keeps a
    /// worktree or a project from being removed, as a live session does.
    pub(crate) fn live_folders(&self) -> Vec<(Occupant, PathBuf)> {
        self.state()
            .entries
            .values()
            .filter(|entry| !entry.handle.is_ended())
            .map(|entry| {
                let info = entry.handle.info();
                (Occupant::Terminal(info.id), info.folder)
            })
            .collect()
    }

    /// The sessions live terminals are beside.
    pub(crate) fn beside(&self) -> Vec<SessionId> {
        let mut sessions: Vec<SessionId> = self
            .state()
            .entries
            .values()
            .filter_map(|entry| entry.handle.info().place.session())
            .collect();
        sessions.sort_unstable();
        sessions.dedup();
        sessions
    }

    /// The live run of `script` in `root`, if there is one.
    pub(crate) fn live_run(&self, root: &Path, script: &ScriptName) -> Option<TerminalId> {
        self.state().entries.values().find_map(|entry| {
            let plan = entry.plan.as_ref()?;
            (!entry.handle.is_ended() && plan.root == root && &plan.entry.name == script)
                .then(|| entry.handle.id())
        })
    }

    /// The runs whose root is `root`.
    pub(crate) fn runs_in(&self, root: &PathBuf) -> Vec<TerminalInfo> {
        let mut runs: Vec<TerminalInfo> = self
            .state()
            .entries
            .values()
            .filter(|entry| entry.plan.as_ref().is_some_and(|plan| &plan.root == root))
            .map(|entry| entry.handle.info())
            .collect();
        runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        runs
    }

    /// The process ids at the roots of the live terminals' trees.
    pub(crate) fn roots(&self) -> Vec<(u32, TerminalId)> {
        self.state()
            .entries
            .values()
            .filter_map(|entry| Some((entry.handle.pid()?, entry.handle.id())))
            .collect()
    }

    /// End and forget every terminal beside `session` (its deletion, or
    /// retention dropping it), and wait for their ends.
    pub(crate) async fn end_session(&self, session: SessionId) {
        let handles = self.handles(|info| info.place.session() == Some(session));
        self.end_all(&handles, true).await;
    }

    /// End (and keep) the terminals an agent opened: its session ended,
    /// and its credential with it.
    pub(crate) async fn end_agent(&self, agent: (NodeId, SessionId)) {
        let handles = self.handles(|info| {
            info.created_by
                == Creator::Agent {
                    node: agent.0,
                    session: agent.1,
                }
        });
        self.end_all(&handles, false).await;
    }

    /// Forget the ended terminals placed in `project` (its removal; a live
    /// one refuses the removal first).
    pub(crate) fn forget_project(&self, project: ProjectId) {
        self.state().entries.retain(|_, entry| {
            let info = entry.handle.info();
            info.is_live() || info.place.project() != Some(project)
        });
    }

    async fn end_all(&self, handles: &[TerminalHandle], forget: bool) {
        for handle in handles {
            handle.kill();
        }
        for handle in handles {
            if tokio::time::timeout(super::scripts::CLOSE_WAIT, handle.ended())
                .await
                .is_err()
            {
                tracing::warn!(terminal = %handle.id(), "a terminal did not end in time");
            }
            if forget {
                self.forget(handle.id());
            }
        }
    }
}
