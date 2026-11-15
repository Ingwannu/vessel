//! One visible project's state. Keys survive screen reopen; no write is retried.
use super::{PreviewHost, PreviewProblem, PreviewUrl, latest_run};
use crate::connection::ConnectionContext;
use crate::layout::{LayoutKey, TabId};
use crate::workspace::command_changes::Listed;
use crate::workspace::git::{Reading, RequestKey, RequestKeys};
use crate::workspace::problem::Problem;
use vessel_proto::TerminalId;
use vessel_proto::ports::PortList;
use vessel_proto::scripts::{ScriptList, ScriptName, ScriptPlace, ScriptRefusal, ScriptRunOutcome};

/// Presentation routing, independent of the node resource or its admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Surface {
    /// The project settings card.
    Project,
    /// Owner state follows the tab across pane moves.
    Panel {
        /// Exact layout owner; a tab id alone is not globally unique.
        workspace: LayoutKey,
        /// Presentation identity, never process ownership.
        tab: TabId,
        /// App-only owner incarnation, preventing a forgotten layout's reused
        /// tab number from accepting an earlier panel's asynchronous receipt.
        instance: RequestKey,
    },
}

/// A displayed resource's identity, never just a mutable selection index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Exact authenticated alias, node and admission captured by the screen.
    pub context: ConnectionContext,
    /// Existing node API resource; no cwd-based ownership inference.
    pub place: ScriptPlace,
    /// App-only destination for correlated replies.
    pub surface: Surface,
}

impl Target {
    /// Bridge only an explicit legacy session layout. Registry workspaces need
    /// their reviewed service contract; they never fall back to selected cwd.
    #[must_use]
    pub fn legacy_panel(context: ConnectionContext, surface: Surface) -> Option<Self> {
        let Surface::Panel {
            workspace: LayoutKey::Session { machine, session },
            ..
        } = &surface
        else {
            return None;
        };
        if machine.as_str() != context.machine().as_str() {
            return None;
        }
        Some(Self {
            context,
            place: ScriptPlace::Session { session: *session },
            surface,
        })
    }
    /// Original alias; it never resolves a replacement admission.
    #[must_use]
    pub fn machine(&self) -> &str {
        self.context.machine().as_str()
    }
}

/// Facts read independently; an unavailable port scanner does not hide scripts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Optional admitted-route suggestion. The current adapter supplies None;
    /// a mutable saved-book alias is not an admitted route preimage.
    pub host: Option<PreviewHost>,
    /// Node-approved configuration and retained supervised runs.
    pub scripts: ScriptList,
    /// Actual process listeners, or why discovery failed.
    pub ports: Result<PortList, Problem>,
}

/// A preview is tied to the exact terminal and observed socket drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// Supervisor resource identity, which can survive a service restart.
    pub terminal: TerminalId,
    /// Complete observed socket, checked again before the browser opens.
    pub listener: vessel_proto::ports::ListeningPort,
    // A UI event fence only; this is not a service lifetime or Preview grant.
    revision: u64,
}

impl Preview {
    /// Capture the terminal publication drawn with the listener. Restart keeps a
    /// live service's `TerminalId`, so that id alone cannot identify this callback.
    #[must_use]
    pub fn observed(
        run: &vessel_proto::terminals::TerminalInfo,
        listener: vessel_proto::ports::ListeningPort,
    ) -> Self {
        Self {
            terminal: run.id,
            listener,
            revision: run.revision,
        }
    }
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

/// A drawn action is tied to the exact successful read as well as its owner.
/// This local correlation ticket grants no node/workspace authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionTicket {
    /// Captured original admission, place and presentation.
    pub target: Target,
    read: RequestKey,
}

impl ActionTicket {
    /// Scope widget identity to the captured receipt and panel incarnation.
    /// This is presentation only and creates no new node permission.
    #[must_use]
    pub fn element_key(&self) -> String {
        format!("{:?}-{:?}", self.target.surface, self.read)
    }
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
    /// Existing sent requests must be retained before another view can write.
    Retention,
}

/// Request correlation belongs to the area's lifetime, beyond detail recreation.
#[derive(Debug, Default)]
pub struct Model {
    keys: RequestKeys,
    // Display-only after a write: Ready remains the sole action authority.
    retained: Option<Snapshot>,
    visible_key: Option<RequestKey>,
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
    /// Withdraw every UI admission synchronously. A sent mutation is returned
    /// intact to its owner; retiring a screen does not cancel or replay it.
    pub fn retire(&mut self) -> Option<super::RetiredWrite> {
        let pending = self.writing.take().and_then(|(key, action)| {
            self.target
                .clone()
                .map(|target| super::RetiredWrite::new(target, key, action))
        });
        self.select(None);
        pending
    }
    /// Change visible owner while preserving the monotonic key issuer.
    pub fn select(&mut self, target: Option<Target>) {
        self.target = target;
        self.read = Reading::Empty;
        self.retained = None;
        self.visible_key = None;
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
        let result = result.map(|mut snapshot| {
            if let ScriptPlace::Session { session } = target.place {
                snapshot
                    .scripts
                    .runs
                    .retain(|run| run.place.session() == Some(session));
            }
            snapshot
        });
        let accepted = self.read.accept(key, result);
        if accepted && self.read.ready().is_some() {
            self.retained = None;
            self.visible_key = Some(key);
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
    /// Capture only a successful, settled display, never retained/loading facts.
    #[must_use]
    pub fn action_ticket(&self) -> Option<ActionTicket> {
        let _snapshot = self.read.ready()?;
        if self.writing.is_some() {
            return None;
        }
        Some(ActionTicket {
            target: self.target.clone()?,
            read: self.visible_key?,
        })
    }
    /// A refreshed command with the same name must not inherit an old button.
    pub fn begin_write_at(&mut self, ticket: &ActionTicket, action: Action) -> Option<RequestKey> {
        if self.target.as_ref() != Some(&ticket.target) || self.visible_key != Some(ticket.read) {
            return None;
        }
        self.begin_write(action)
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
        if run.revision != preview.revision
            || super::run_state(Some(run)) != super::RunState::Running
        {
            return Err(PreviewProblem::Stale);
        }
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
