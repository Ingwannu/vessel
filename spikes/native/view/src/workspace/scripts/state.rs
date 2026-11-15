//! One visible project's state. Keys survive screen reopen; no write is retried.
use super::{PreviewHost, PreviewProblem, PreviewUrl, latest_run};
use crate::machine_book::Name;
use crate::workspace::command_changes::Listed;
use crate::workspace::git::{Reading, RequestKey, RequestKeys};
use crate::workspace::problem::Problem;
use vessel_proto::ports::PortList;
use vessel_proto::scripts::{ScriptList, ScriptName, ScriptRefusal, ScriptRunOutcome};
use vessel_proto::{ProjectId, TerminalId};

/// A displayed project's identity, never just a mutable selection index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Validated node-book name.
    pub machine: Name,
    /// Project whose folder runs these commands.
    pub project: ProjectId,
}

/// Facts read independently; an unavailable port scanner does not hide scripts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Suggested direct host from the saved book; never a relay endpoint.
    pub host: Option<PreviewHost>,
    /// Node-approved configuration and retained supervised runs.
    pub scripts: ScriptList,
    /// Actual process listeners, or why discovery failed.
    pub ports: Result<PortList, Problem>,
}

/// A preview is tied to the exact terminal and observed socket drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// Supervisor identity; a restarted service is a different intention.
    pub terminal: TerminalId,
    /// Complete observed socket, checked again before the browser opens.
    pub listener: vessel_proto::ports::ListeningPort,
}

/// An intention is checked against the current snapshot before it crosses link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Run a configured command from its approved file.
    Run(ScriptName),
    /// Restart the exact supervised run, never a shell selected by name.
    Restart(TerminalId),
    /// Stop the exact supervised run.
    Stop(TerminalId),
}

/// Persistent, typed explanation shown until the next explicit intention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Issue {
    /// A request could not complete.
    Call(Problem),
    /// The exact command set needs a fresh person-approved digest.
    Approval,
    /// The node refused an otherwise valid intention.
    Refused(ScriptRefusal),
    /// Browser destination was missing or invalid.
    Preview(PreviewProblem),
    /// Monotonic request identities must never wrap.
    Exhausted,
}

/// Request correlation belongs to the area's lifetime, beyond detail recreation.
#[derive(Debug, Default)]
pub struct Model {
    keys: RequestKeys,
    // Display-only after a write: Ready remains the sole action authority.
    retained: Option<Snapshot>,
    /// Which detail owns these facts.
    pub target: Option<Target>,
    /// A failed or loading read cannot authorize a new write.
    pub read: Reading<Snapshot>,
    /// Only this mutation can settle a pending intention.
    pub writing: Option<(RequestKey, Action)>,
    /// A visible failure, separate from background refresh.
    pub issue: Option<Issue>,
}

impl Model {
    /// Change visible owner while preserving the monotonic key issuer.
    pub fn select(&mut self, target: Option<Target>) {
        self.target = target;
        self.read = Reading::Empty;
        self.retained = None;
        self.writing = None;
        self.issue = None;
    }
    /// Start at most one read, and never race an in-flight mutation.
    pub fn begin_read(&mut self) -> Option<RequestKey> {
        if self.target.is_none() || self.read.outstanding().is_some() || self.writing.is_some() {
            return None;
        }
        let key = self.key()?;
        self.read.start(key);
        Some(key)
    }
    /// Ignore replies from another machine/project, older opens and duplicates.
    pub fn accept_read(
        &mut self,
        target: &Target,
        key: RequestKey,
        result: Result<Snapshot, Problem>,
    ) -> bool {
        if self.target.as_ref() != Some(target) {
            return false;
        }
        let accepted = self.read.accept(key, result);
        if accepted && self.read.ready().is_some() {
            self.retained = None;
        }
        accepted
    }
    /// Preserve the card's geometry during post-write reconciliation. This value
    /// never grants run, stop, restart or browser permission; those require Ready.
    #[must_use]
    pub fn shown(&self) -> Option<&Snapshot> {
        self.read.shown().or(self.retained.as_ref())
    }
    /// Check configured names, approval and exact terminal ownership afresh.
    #[must_use]
    pub fn permits(&self, action: &Action) -> bool {
        if self.writing.is_some() {
            return false;
        }
        let Some(snapshot) = self.read.ready() else {
            return false;
        };
        match action {
            Action::Run(name) => Listed::of(&snapshot.scripts.commands).is_some_and(|listed| {
                listed.approved
                    && listed.scripts.iter().any(|entry| &entry.name == name)
                    && latest_run(name, &snapshot.scripts.runs)
                        .is_none_or(|run| !run.status.is_live())
            }),
            Action::Restart(id) | Action::Stop(id) => snapshot.scripts.runs.iter().any(|run| {
                run.id == *id
                    && run.kind.script().is_some()
                    && run.status.is_live()
                    && run.kind.script().is_some_and(|name| {
                        latest_run(name, &snapshot.scripts.runs)
                            .is_some_and(|latest| latest.id == *id)
                    })
            }),
        }
    }
    /// Mark pending only after validation, so duplicate presses send nothing.
    pub fn begin_write(&mut self, action: Action) -> Option<RequestKey> {
        if !self.permits(&action) {
            return None;
        }
        let key = self.key()?;
        self.writing = Some((key, action));
        self.issue = None;
        Some(key)
    }
    /// Apply an exact receipt, then require a read before any next intention.
    pub fn accept_write(
        &mut self,
        target: &Target,
        key: RequestKey,
        result: Result<Option<ScriptRunOutcome>, Problem>,
    ) -> bool {
        if self.target.as_ref() != Some(target)
            || !self
                .writing
                .as_ref()
                .is_some_and(|(pending, _)| *pending == key)
        {
            return false;
        }
        self.writing = None;
        self.retained = self.read.shown().cloned();
        self.read = Reading::Empty;
        self.issue = match result {
            Err(problem) => Some(Issue::Call(problem)),
            Ok(Some(ScriptRunOutcome::NeedsApproval { .. })) => Some(Issue::Approval),
            Ok(Some(ScriptRunOutcome::Refused { reason })) => Some(Issue::Refused(reason)),
            Ok(Some(ScriptRunOutcome::Started { .. }) | None) => None,
        };
        true
    }
    /// Recheck freshness and process ownership at the final browser boundary.
    /// # Errors
    /// Refuses stale sockets, invalid hosts and remote loopback destinations.
    pub fn preview_url(
        &self,
        preview: &Preview,
        explicit_host: &str,
    ) -> Result<PreviewUrl, PreviewProblem> {
        let snapshot = self
            .read
            .ready()
            .filter(|_| self.writing.is_none())
            .ok_or(PreviewProblem::Stale)?;
        // The original scan failure stays in Snapshot for its own notice;
        // opening a browser additionally requires a fresh successful scan.
        let ports = snapshot
            .ports
            .as_ref()
            .map_err(|_scan_failure| PreviewProblem::Stale)?;
        let run = snapshot
            .scripts
            .runs
            .iter()
            .find(|run| run.id == preview.terminal)
            .ok_or(PreviewProblem::Stale)?;
        let latest = run
            .kind
            .script()
            .and_then(|name| latest_run(name, &snapshot.scripts.runs));
        if !matches!(ports.scan, vessel_proto::ports::PortScan::Scanned { .. })
            || latest.is_none_or(|latest| latest.id != run.id)
            || !super::listeners(Some(run), ports).contains(&&preview.listener)
        {
            return Err(PreviewProblem::Stale);
        }
        let host = if explicit_host.is_empty() {
            snapshot.host.clone().ok_or(PreviewProblem::MissingHost)?
        } else {
            PreviewHost::parse(explicit_host)?
        };
        if preview.listener.address.is_loopback() && !host.is_loopback() {
            return Err(PreviewProblem::RemoteLoopback);
        }
        host.url(preview.listener.port)
    }
    fn key(&mut self) -> Option<RequestKey> {
        match self.keys.issue() {
            Ok(key) => Some(key),
            Err(_exhausted) => {
                self.issue = Some(Issue::Exhausted);
                None
            }
        }
    }
}
