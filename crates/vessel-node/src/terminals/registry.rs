//! [`Terminals`]: the node's terminals by id, their limits, and what ties
//! them to sessions, projects and agents.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use vessel_proto::scripts::ScriptName;
use vessel_proto::terminals::TerminalInfo;
use vessel_proto::{ErrorCode, NodeId, ProjectId, SessionId, TerminalId, WireError};
use vessel_session::terminals::TerminalHandle;

use super::scripts::Plan;
use super::{MAX_ENDED, MAX_LIVE, MAX_PER_AGENT, MAX_PER_PROJECT, MAX_PER_SESSION, Runner};

mod attention;
mod ending;
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

/// A terminal as the node keeps it.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) handle: TerminalHandle,
    /// What a script's run ran, to run it again.
    pub(crate) plan: Option<Arc<Plan>>,
    /// What it counts against, as it was reserved (a terminal beside a
    /// session in a project counts against the project too, which its
    /// place alone does not say).
    pub(crate) owner: Owner,
    producer: Option<producers::Producer>,
}

/// A start to count before it runs.
#[derive(Debug, Clone)]
pub(crate) struct Start {
    /// What it counts against.
    pub(crate) owner: Owner,
    /// The id it will have.
    pub(crate) id: TerminalId,
    /// The folder it will run in: a start in progress keeps a worktree
    /// or a project from being removed, as a live terminal does.
    pub(crate) folder: PathBuf,
    /// A script's run: its checkout and name, of which one runs at a time.
    pub(crate) run: Option<(PathBuf, ScriptName)>,
}

#[derive(Debug, Default)]
struct State {
    entries: HashMap<TerminalId, Entry>,
    /// Starts in progress, counted with the live ones.
    starting: Vec<(u64, Start)>,
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
    /// The OS random source could not mint a terminal activity capability.
    #[error("the terminal activity credential could not be created")]
    Random(#[source] getrandom::Error),
    /// The shell did not start.
    #[error("the shell could not be started: {0}")]
    Spawn(String),
    /// The node is stopping.
    #[error("the node is stopping")]
    ShuttingDown,
    /// A failure already on the wire's terms (a place's, a session's).
    #[error("{}", .0.message)]
    Wire(WireError),
    /// The script runs already (or is starting), in this terminal.
    #[error("the script runs already, in terminal {0}")]
    Running(TerminalId),
}

impl TerminalsError {
    /// The error a client sees.
    pub(crate) fn wire(self) -> WireError {
        let code = match &self {
            Self::Random(_) => ErrorCode::Internal,
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Limit(_) | Self::ShuttingDown => ErrorCode::Unavailable,
            Self::Forbidden(_) => ErrorCode::Forbidden,
            Self::Conflict(_) | Self::Running(_) => ErrorCode::Conflict,
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
    /// The sessions whose end is watched for agents' terminals, one task
    /// each however many terminals.
    watching: Mutex<HashSet<SessionId>>,
    attention_changes: tokio::sync::watch::Sender<()>,
}

/// A start counted against the limits until it is registered or dropped.
#[derive(Debug)]
pub(crate) struct Reservation {
    state: Arc<Mutex<State>>,
    number: u64,
    owner: Owner,
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

fn lock_set(set: &Mutex<HashSet<SessionId>>) -> MutexGuard<'_, HashSet<SessionId>> {
    set.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Terminals {
    pub(crate) fn new(runner: Runner) -> Arc<Self> {
        Arc::new(Self {
            runner,
            state: Arc::default(),
            watching: Mutex::default(),
            attention_changes: tokio::sync::watch::Sender::new(()),
        })
    }

    fn state(&self) -> MutexGuard<'_, State> {
        lock(&self.state)
    }

    /// Count a start against the limits, or say which is full; for a
    /// script's run, also that it does not run already. One lock for all
    /// of it, so starts at once cannot all pass.
    ///
    /// # Errors
    /// `Limit` (the node's, a session's, a project's), `Forbidden` (an
    /// agent's four) or `Running` (the script's live or starting run).
    pub(crate) fn reserve(&self, start: Start) -> Result<Reservation, TerminalsError> {
        let mut state = self.state();
        if let Some((root, script)) = &start.run {
            let live = state.entries.values().find_map(|entry| {
                let plan = entry.plan.as_ref()?;
                (!entry.handle.is_ended() && &plan.root == root && &plan.entry.name == script)
                    .then(|| entry.handle.id())
            });
            let starting = state
                .starting
                .iter()
                .find(|(_, other)| {
                    other
                        .run
                        .as_ref()
                        .is_some_and(|(their_root, theirs)| their_root == root && theirs == script)
                })
                .map(|(_, other)| other.id);
            if let Some(terminal) = live.or(starting) {
                return Err(TerminalsError::Running(terminal));
            }
        }
        let owner = start.owner.clone();
        let live: Vec<Owner> = state
            .entries
            .values()
            .filter(|entry| !entry.handle.is_ended())
            .map(|entry| entry.owner.clone())
            .chain(state.starting.iter().map(|(_, other)| other.owner.clone()))
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
        state.starting.push((number, start));
        Ok(Reservation {
            state: Arc::clone(&self.state),
            number,
            owner,
        })
    }

    /// Keep a started terminal (its reservation ends, its owner kept),
    /// forgetting the oldest ended ones past the bound. Synchronous, so a
    /// caller holding the handle registers it before it can stop waiting.
    pub(crate) fn register(
        &self,
        reservation: Reservation,
        handle: TerminalHandle,
        plan: Option<Arc<Plan>>,
    ) {
        let id = handle.id();
        let changes = handle.watch();
        let entry = Entry {
            handle,
            plan,
            owner: reservation.owner.clone(),
            producer: None,
        };
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
        self.watch_attention(changes);
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
        if let Some(entry) = self.state().entries.remove(&id)
            && let Some(producer) = entry.producer
        {
            producer.close();
        }
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

    /// The live terminals, and those starting, with the folders they run
    /// in: each keeps a worktree or a project from being removed, as a
    /// live session does.
    pub(crate) fn live_folders(&self) -> Vec<(Occupant, PathBuf)> {
        let state = self.state();
        state
            .entries
            .values()
            .filter(|entry| !entry.handle.is_ended())
            .map(|entry| {
                let info = entry.handle.info();
                (Occupant::Terminal(info.id), info.folder)
            })
            .chain(
                state
                    .starting
                    .iter()
                    .map(|(_, start)| (Occupant::Terminal(start.id), start.folder.clone())),
            )
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

    /// Forget the ended terminals placed in `project` (its removal; a live
    /// one refuses the removal first).
    pub(crate) fn forget_project(&self, project: ProjectId) {
        self.state().entries.retain(|_, entry| {
            let info = entry.handle.info();
            info.is_live() || info.place.project() != Some(project)
        });
    }
}

mod producers;
