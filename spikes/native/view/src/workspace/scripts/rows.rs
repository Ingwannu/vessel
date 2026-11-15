//! Correlate configured names with their latest supervised run and ports.
use crate::workspace::command_changes::Listed;
use vessel_proto::SessionStatus;
use vessel_proto::ports::{ListeningPort, PortList, PortOwner};
use vessel_proto::scripts::ScriptList;
use vessel_proto::scripts::{ScriptEntry, ScriptEntryView, ScriptKind, ScriptName};
use vessel_proto::terminals::{ServiceState, TerminalInfo, TerminalKind, TerminalPlace};
use vessel_proto::workspace::{ProjectCommand, RelPath, ServicePort};

/// Complete display projection of the node's approved declaration.
#[derive(Debug, Clone, Copy)]
pub struct Entry<'a> {
    /// Exact configured name.
    pub name: &'a ScriptName,
    /// Execution role, independent from observed process state.
    pub kind: ScriptKind,
    /// Exact approved command.
    pub command: &'a ProjectCommand,
    /// Configured working directory relative to the project.
    pub dir: Option<&'a RelPath>,
    /// Approved declaration, never an observed listener or health result.
    pub port: Option<ServicePort>,
    /// Actual process directory for a retained run, separate from declaration.
    pub observed_dir: Option<&'a std::path::Path>,
}
impl<'a> Entry<'a> {
    /// Name every approved field so future additions break the adapter.
    #[must_use]
    pub fn of(entry: &'a ScriptEntry) -> Self {
        let ScriptEntryView {
            name,
            kind,
            command,
            dir,
            port,
        } = entry.view();
        Self {
            name,
            kind,
            command,
            dir,
            port,
            observed_dir: None,
        }
    }
}

/// Lifecycle words have one typed meaning across scripts and services.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    /// No run is retained.
    Idle,
    /// Process creation is in progress.
    Starting,
    /// Process is alive; this does not claim health.
    Running,
    /// Supervisor is waiting before retrying a failed service.
    Backoff,
    /// Process ended, with its optional exit code.
    Exited(Option<i32>),
    /// Creation or supervision failed.
    Failed,
    /// A person or shutdown stopped it.
    Stopped,
    /// A newer node reports a state this app cannot interpret.
    Unknown,
}

/// Select by creation time and id, never by reply order or an older run.
#[must_use]
pub fn latest_run<'a>(name: &ScriptName, runs: &'a [TerminalInfo]) -> Option<&'a TerminalInfo> {
    runs.iter()
        .filter(|run| run.kind.script() == Some(name))
        .max_by_key(|run| (run.created_at, run.id))
}

/// Service supervision takes precedence over its still-live terminal status.
#[must_use]
pub fn run_state(run: Option<&TerminalInfo>) -> RunState {
    let Some(run) = run else {
        return RunState::Idle;
    };
    if let Some(service) = &run.service {
        return match service {
            ServiceState::Starting { .. } => RunState::Starting,
            ServiceState::Running { .. } => RunState::Running,
            ServiceState::BackingOff { .. } => RunState::Backoff,
            ServiceState::Exited { code } => RunState::Exited(Some(*code)),
            ServiceState::Failed { .. } => RunState::Failed,
            ServiceState::Stopped => RunState::Stopped,
            ServiceState::Unknown => RunState::Unknown,
        };
    }
    match &run.status {
        SessionStatus::Starting => RunState::Starting,
        SessionStatus::Running => RunState::Running,
        SessionStatus::Exited { code } => RunState::Exited(*code),
        SessionStatus::Failed { .. } => RunState::Failed,
        SessionStatus::Interrupted => RunState::Stopped,
    }
}

/// Only the selected live run's listeners belong under a configured script.
/// An old run's ports must not authorize a preview after restart or stop.
#[must_use]
pub fn listeners<'a>(run: Option<&TerminalInfo>, ports: &'a PortList) -> Vec<&'a ListeningPort> {
    let Some(run) = run.filter(|run| run.status.is_live()) else {
        return Vec::new();
    };
    ports
        .ports
        .iter()
        .filter(|port| match port.owner {
            PortOwner::Terminal { terminal } => terminal == run.id,
            PortOwner::Session { .. } => false,
        })
        .collect()
}

/// Keep a supervised run reachable even if its file entry was removed or broken.
#[must_use]
pub fn retained_runs(list: &ScriptList) -> Vec<&TerminalInfo> {
    let configured = Listed::of(&list.commands);
    list.runs
        .iter()
        .filter(|run| run.status.is_live())
        .filter(|run| {
            run.kind.script().is_some_and(|name| {
                !configured
                    .as_ref()
                    .is_some_and(|listed| listed.scripts.iter().any(|entry| &entry.name == name))
                    && latest_run(name, &list.runs).is_some_and(|latest| latest.id == run.id)
            })
        })
        .collect()
}
impl<'a> Entry<'a> {
    /// Historical process facts are display-only; Run still checks current approval.
    #[must_use]
    pub fn retained(run: &'a TerminalInfo) -> Option<Self> {
        let (name, kind, command) = match &run.kind {
            TerminalKind::Script { script, command } => (script, ScriptKind::Script, command),
            TerminalKind::Service { script, command } => (script, ScriptKind::Service, command),
            TerminalKind::Shell { .. } | TerminalKind::Unknown => return None,
        };
        let dir = match &run.place {
            TerminalPlace::Project { dir, .. } => dir.as_ref(),
            TerminalPlace::Session { .. } => None,
        };
        Some(Self {
            name,
            kind,
            command,
            dir,
            port: None,
            observed_dir: Some(&run.folder),
        })
    }
}
