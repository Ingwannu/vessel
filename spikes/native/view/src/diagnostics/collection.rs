//! One explicit collection run, with a hard combined text budget.
//! Refresh and close replace the run identity, so late replies cannot revive it.

use super::DaemonSnapshot;
use crate::board::{Machine, Reach};
use crate::connection::ConnectionTag;
use crate::devices::{ConnectionContext, MachineKey, Ticket};
use vessel_proto::{NodeId, NodeState, ProtocolVersion};

/// Maximum combined UTF-8 report bytes accepted for display or copying.
pub const MAX_REPORT_BYTES: usize = 128 * 1024;
/// Maximum host rows retained by a diagnostic run.
pub const MAX_REPORT_HOSTS: usize = 64;

/// A saved-host count, separate from an identifier or byte budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostCount(usize);
impl HostCount {
    /// Carry the length of an already parsed node book.
    #[must_use]
    pub const fn from_book(count: usize) -> Self {
        Self(count)
    }
    /// Numeric count for the technical report header.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// Shareable text that has passed redaction and the combined output budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportText(std::sync::Arc<str>);
impl ReportText {
    /// Sanitize before admitting text to the copyable report.
    /// # Errors
    /// Refuses an oversized report rather than truncating it into apparent completeness.
    pub fn new(text: &str) -> Result<Self, Problem> {
        if text.len() > MAX_REPORT_BYTES {
            return Err(Problem::TooLarge);
        }
        let text = super::redact(text);
        if text.len() > MAX_REPORT_BYTES {
            return Err(Problem::TooLarge);
        }
        Ok(Self(text.into()))
    }
    /// Text suitable for the report viewer and clipboard.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A failure category; remote strings and causes never enter the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// No connection at request time.
    #[error("host is not connected")]
    Offline,
    /// An earlier connection can no longer supply this report.
    #[error("host connection changed; run the report again")]
    ConnectionChanged,
    /// A bounded collector has no free permit.
    #[error("diagnostics is busy")]
    Busy,
    /// The node is shutting down and refuses new collection work.
    #[error("node is stopping")]
    Stopping,
    /// A producer has not been registered.
    #[error("diagnostic API is not available in this build")]
    Unavailable,
    /// The peer predates this operation.
    #[error("the host does not support this diagnostic")]
    Unsupported,
    /// Read authority was refused.
    #[error("the host refused diagnostic access")]
    Forbidden,
    /// The read did not finish before its named deadline.
    #[error("diagnostic collection timed out")]
    TimedOut,
    /// A malformed or mismatched identity was returned.
    #[error("the host returned an invalid diagnostic")]
    InvalidReply,
    /// A read failed without exposing its cause text.
    #[error("diagnostic collection failed")]
    Failed,
    /// No partial report may pretend to be the complete bounded report.
    #[error("the combined diagnostic report is too large")]
    TooLarge,
    /// Identity allocation has exhausted its finite range.
    #[error("diagnostic request identities are exhausted")]
    Exhausted,
}

/// Facts projected from the existing node.info read, with no peer-controlled text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeFacts {
    /// Identity admitted by the existing authenticated connection.
    pub node: NodeId,
    /// Negotiated protocol, not an advertised arbitrary version string.
    pub protocol: ProtocolVersion,
    /// Whether the node accepts new sessions.
    pub state: NodeState,
}

/// A host read retains the expected authenticated identity independently of its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostResult {
    /// Connection identity, known even when the fresh read fails.
    pub admitted: NodeId,
    /// A fresh safe projection.
    pub facts: Result<NodeFacts, Problem>,
    /// Independent safe daemon result, even if the basic node read failed.
    pub daemon: DaemonSnapshot,
}

/// Every row remains explicit until answered; no missing source becomes zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    /// A request has not yet been dispatched.
    Pending,
    /// A request is in flight.
    Running,
    /// A successful safe projection; full daemon diagnostics remain independent.
    Done(HostResult),
    /// Independent sources settled but some facts remain unavailable.
    Partial(HostResult),
    /// The source failed or is unavailable.
    Failed(Problem),
}

/// One node-book entry in a run. Its label is UI-only, never copied as report content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRow {
    /// Existing parsed book identity, used solely for correlation.
    pub machine: MachineKey,
    /// Welcome identity pinned for this run, or the first admitted reply when initially unknown.
    pub admitted: Option<NodeId>,
    /// Shared physical admission, pinned before any app dispatch.
    pub context: Option<ConnectionContext>,
    /// Current request outcome.
    pub progress: Progress,
}

/// The visible lifecycle of the diagnostic sheet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Phase {
    /// No pending result may reopen the sheet.
    #[default]
    Hidden,
    /// The client result is ready and host reads remain.
    Collecting,
    /// Every requested host settled, including unavailable sources.
    Ready,
    /// The combined result was refused.
    Failed(Problem),
}

/// A complete run; pure state, no I/O, UI toolkit or node producer.
#[derive(Debug, Default)]
pub struct Collection {
    ticket: Option<Ticket>,
    phase: Phase,
    client: Option<ReportText>,
    rows: Vec<HostRow>,
    report: Option<ReportText>,
}
impl Collection {
    /// Supersede the previous run before admitting its new host list.
    pub fn start(
        &mut self,
        ticket: Ticket,
        client: ReportText,
        hosts: impl IntoIterator<Item = (MachineKey, Result<(), Problem>)>,
    ) {
        self.close();
        self.ticket = Some(ticket);
        self.client = Some(client);
        for (machine, availability) in hosts {
            if self.rows.iter().any(|row| row.machine == machine) {
                self.phase = Phase::Failed(Problem::InvalidReply);
                self.rows.clear();
                return;
            }
            if self.rows.len() >= MAX_REPORT_HOSTS {
                self.phase = Phase::Failed(Problem::TooLarge);
                self.rows.clear();
                return;
            }
            let progress = match availability {
                Ok(()) => Progress::Pending,
                Err(problem) => Progress::Failed(problem),
            };
            self.rows.push(HostRow {
                machine,
                admitted: None,
                context: None,
                progress,
            });
        }
        self.phase = Phase::Collecting;
        self.finish();
    }
    /// Freeze the shared fleet's welcome identity before dispatch; never retarget an active read.
    pub fn expect_node(&mut self, machine: &MachineKey, node: NodeId) {
        if let Some(row) = self.rows.iter_mut().find(|row| &row.machine == machine)
            && row.progress == Progress::Pending
        {
            row.admitted = Some(node);
        }
    }
    /// Bind the same shared admission used by the link's before-poll guard.
    pub fn expect_connection(&mut self, context: ConnectionContext) {
        self.expect_node(context.machine(), context.node());
        if let Some(row) = self
            .rows
            .iter_mut()
            .find(|row| &row.machine == context.machine())
            && row.progress == Progress::Pending
        {
            row.context = Some(context);
        }
    }
    /// A response from another epoch cannot settle this run, even for the same node.
    pub fn answer_connection(
        &mut self,
        ticket: Ticket,
        context: &ConnectionContext,
        result: Result<HostResult, Problem>,
    ) {
        if self
            .rows
            .iter()
            .any(|row| row.context.as_ref() == Some(context))
        {
            self.answer(ticket, context.machine(), result);
        }
    }
    /// Retirement is ordered before a later Fleet, so old text is removed immediately.
    pub fn retired(&mut self, tag: &ConnectionTag) {
        if self.rows.iter().any(|row| {
            row.context
                .as_ref()
                .is_some_and(|context| context.tag() == *tag)
        }) {
            self.invalidate(tag.machine());
        }
    }
    /// Mark only the pending row of the current run as dispatched.
    pub fn running(&mut self, ticket: Ticket, machine: &MachineKey) {
        if self.ticket != Some(ticket) {
            return;
        }
        if let Some(row) = self.rows.iter_mut().find(|row| &row.machine == machine)
            && row.progress == Progress::Pending
        {
            row.progress = Progress::Running;
        }
    }
    /// Accept exactly one answer for a running row in the active run.
    pub fn answer(
        &mut self,
        ticket: Ticket,
        machine: &MachineKey,
        result: Result<HostResult, Problem>,
    ) {
        if self.ticket != Some(ticket) || self.phase != Phase::Collecting {
            return;
        }
        let Some(row) = self.rows.iter_mut().find(|row| &row.machine == machine) else {
            return;
        };
        if row.progress != Progress::Running {
            return;
        }
        row.progress = match result {
            Ok(answer) => {
                let wrong_admission = row.admitted.is_some_and(|node| node != answer.admitted);
                let wrong_basic = answer
                    .facts
                    .as_ref()
                    .is_ok_and(|facts| facts.node != answer.admitted);
                let wrong_report = answer
                    .daemon
                    .node()
                    .is_some_and(|node| node != answer.admitted);
                if wrong_admission || wrong_basic || wrong_report {
                    Progress::Failed(Problem::InvalidReply)
                } else {
                    row.admitted = Some(answer.admitted);
                    match (&answer.facts, &answer.daemon) {
                        (Err(problem), DaemonSnapshot::Unavailable(_)) => {
                            Progress::Failed(*problem)
                        }
                        (Ok(_), DaemonSnapshot::Report(_)) if answer.daemon.complete() => {
                            Progress::Done(answer)
                        }
                        (
                            Ok(_) | Err(_),
                            DaemonSnapshot::Unavailable(_) | DaemonSnapshot::Report(_),
                        ) => Progress::Partial(answer),
                    }
                }
            }
            Err(problem) => Progress::Failed(problem),
        };
        self.finish();
    }
    /// A new connection is a new observation generation, including the same node id.
    pub fn connected(&mut self, machine: &MachineKey, node: NodeId) {
        if let Some(row) = self.rows.iter_mut().find(|row| &row.machine == machine) {
            row.admitted = Some(node);
            row.progress = Progress::Failed(Problem::ConnectionChanged);
            self.report = None;
            self.finish();
        }
    }
    /// Reconcile the shared fleet identity before any pending or completed facts are drawn.
    pub fn reconcile(&mut self, machines: &[Machine]) {
        let changes: Vec<_> = self
            .rows
            .iter()
            .filter_map(|row| {
                match machines
                    .iter()
                    .find(|machine| machine.name == row.machine.as_str())
                {
                    Some(machine)
                        if row.context.as_ref().is_some_and(|old| {
                            machine.connection_context().as_ref() != Some(old)
                        }) =>
                    {
                        Some(FleetChange::Offline(row.machine.clone()))
                    }
                    Some(_machine) if row.context.is_some() => None,
                    Some(machine) => match &machine.reach {
                        Reach::Online => machine
                            .node
                            .filter(|node| row.admitted.is_some_and(|old| old != *node))
                            .map(|node| FleetChange::Connected(row.machine.clone(), node)),
                        Reach::Unreachable(_) => Some(FleetChange::Offline(row.machine.clone())),
                    },
                    None => Some(FleetChange::Offline(row.machine.clone())),
                }
            })
            .collect();
        for change in changes {
            match change {
                FleetChange::Connected(machine, node) => self.connected(&machine, node),
                FleetChange::Offline(machine) => self.invalidate(&machine),
            }
        }
    }
    /// A renamed/forgotten/offline entry cannot retain a pending old-connection result.
    pub fn invalidate(&mut self, machine: &MachineKey) {
        if let Some(row) = self.rows.iter_mut().find(|row| &row.machine == machine)
            && row.progress != Progress::Failed(Problem::Offline)
        {
            row.progress = Progress::Failed(Problem::Offline);
            self.report = None;
            self.finish();
        }
    }
    /// Refuse a replacement run immediately, invalidating every previous answer.
    pub fn refuse(&mut self, problem: Problem) {
        self.close();
        self.phase = Phase::Failed(problem);
    }
    /// Close immediately; no outstanding answer still names an active run.
    pub fn close(&mut self) {
        self.ticket = None;
        self.phase = Phase::Hidden;
        self.client = None;
        self.rows.clear();
        self.report = None;
    }
    /// An explicit new run waits for the current collection to settle.
    #[must_use]
    pub const fn can_start(&self) -> bool {
        match self.phase {
            Phase::Hidden | Phase::Ready | Phase::Failed(_) => true,
            Phase::Collecting => false,
        }
    }
    /// Correlation for dispatching the next pending host of this same run.
    #[must_use]
    pub const fn ticket(&self) -> Option<Ticket> {
        self.ticket
    }
    /// What the screen draws.
    #[must_use]
    pub const fn phase(&self) -> &Phase {
        &self.phase
    }
    /// Current host rows, in saved-book order.
    #[must_use]
    pub fn rows(&self) -> &[HostRow] {
        &self.rows
    }
    /// Whether this run actually admitted the local client report.
    #[must_use]
    pub const fn has_client_report(&self) -> bool {
        self.client.is_some()
    }
    /// Only a fully settled, bounded report can be copied.
    #[must_use]
    pub const fn report(&self) -> Option<&ReportText> {
        self.report.as_ref()
    }
    fn finish(&mut self) {
        if matches!(self.phase, Phase::Hidden | Phase::Failed(_)) {
            return;
        }
        if self
            .rows
            .iter()
            .any(|row| matches!(row.progress, Progress::Pending | Progress::Running))
        {
            return;
        }
        match super::format::combined(self.client.as_ref(), &self.rows) {
            Ok(report) => {
                self.report = Some(report);
                self.phase = Phase::Ready;
            }
            Err(problem) => {
                self.report = None;
                self.phase = Phase::Failed(problem);
            }
        }
    }
}

#[cfg(test)]
mod tests;

/// A bounded projection of fleet changes; no raw reachability error is retained.
enum FleetChange {
    Connected(MachineKey, NodeId),
    Offline(MachineKey),
}
