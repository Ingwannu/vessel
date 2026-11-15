//! Agent pages keyed by the existing node-book identity. A background
//! install belongs to its machine even while another machine is shown.
//!
//! [Decision Log]
//! - Purpose: do not lose install output on a machine switch, or apply an
//!   old answer to a new request or a different machine.
//! - Constraints: the frame identifies nodes by their validated book key;
//!   the link allows four concurrent tasks; each install retains 1 MiB.
//! - Alternatives: one current-machine record (loses background jobs), an
//!   unbounded cache, or keeping state in GPUI callbacks.
//! - Choice: typed book keys and request tickets, eight cached pages;
//!   only idle pages may be evicted. All transitions are UI-free here.
//! - Cost: an evicted idle page is read again; only recent job output is
//!   retained, rather than inventing a node-side install history.

use super::install::{Completion, Transcript};
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::sync::Arc;
use vessel_proto::agents::{AgentInfo, AgentReadiness, InstallAgent, InstallJob, InstallOutput};
use vessel_proto::{Harness, HarnessRef, RequestId};

/// Cache bound, including active pages. At most eight 1 MiB transcripts.
pub const MAX_PAGES: usize = 8;

/// A node-book key, distinct from a node id, label, agent or profile name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MachineKey(Arc<str>);
impl MachineKey {
    /// Tag a key already read from the validated node book, as the frame does.
    #[must_use]
    pub fn from_book(name: impl Into<Arc<str>>) -> Self {
        Self(name.into())
    }
    /// The key for the link's existing connection map.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Borrow<str> for MachineKey {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

/// Calls supported by the node, rather than a successful-looking empty list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Administration {
    /// Protocol 11 or later.
    Supported,
    /// Discovery only; do not advertise administration controls.
    OlderNode,
}

/// One complete Agents snapshot from the same machine connection.
#[derive(Debug)]
pub struct Inventory {
    /// The node that reported these facts; a reused book name is not identity.
    pub node: vessel_proto::NodeId,
    /// Remote OS-derived command syntax; never use the client platform.
    pub command_shell: super::command::CommandShell,
    /// Discovered installed programs.
    pub harnesses: Vec<Harness>,
    /// All definitions, including missing programs.
    pub agents: Vec<AgentInfo>,
    /// Cached readiness, without an automatic login check.
    pub readiness: Vec<AgentReadiness>,
    /// Calls this node can answer.
    pub administration: Administration,
}

/// A pending operation also determines where any error belongs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Reading the page.
    Discovery,
    /// Checking this agent's default profile.
    Check(HarnessRef),
    /// Running this exact node-offered installer part.
    Install(InstallAgent),
    /// Forgetting this added definition, never stopping its running sessions.
    Remove {
        /// The node whose definition was confirmed.
        node: vessel_proto::NodeId,
        /// The added definition on that node.
        name: vessel_proto::agents::AgentName,
    },
}

/// A request and its meaning stay together.
#[derive(Debug)]
pub struct Ticket {
    /// Link correlation id.
    pub id: RequestId,
    /// Where progress or failure belongs.
    pub operation: Operation,
}

/// A failed action keeps the original typed source.
#[derive(Debug)]
pub struct Issue<E> {
    /// The action that failed.
    pub operation: Operation,
    /// Its unmodified source, for presentation at the boundary.
    pub source: E,
}

/// A job always has its command, identity and retained output together.
#[derive(Debug)]
pub struct InstallLog {
    /// The node's accepted job.
    pub job: InstallJob,
    /// Bytes combined across output pages.
    pub transcript: Transcript,
}

/// An output-read failure is not a failed installer exit.
#[derive(Debug, Default)]
pub enum InstallProgress {
    /// No accepted job to display.
    #[default]
    None,
    /// Reading pages, including terminal pages not yet drained.
    Following(InstallLog),
    /// All retained output was read.
    Finished {
        /// The original job and output.
        log: InstallLog,
        /// The actual terminal state reported by the node.
        completion: Completion,
    },
    /// Output could no longer be read; the installer may still be running.
    Unavailable(InstallLog),
}

/// State on one machine, independent of whichever machine is visible.
#[derive(Debug)]
pub struct Page<E> {
    /// The last successful snapshot, retained while refreshing.
    inventory: Option<Inventory>,
    /// The detail card the person opened here.
    selected: Option<HarnessRef>,
    /// One in-flight operation per machine; the node serializes installs too.
    pending: Option<Ticket>,
    /// The last action's error and its context.
    issue: Option<Issue<E>>,
    /// This machine's install output survives selecting another machine.
    install: InstallProgress,
}
impl<E> Default for Page<E> {
    fn default() -> Self {
        Self {
            inventory: None,
            selected: None,
            pending: None,
            issue: None,
            install: InstallProgress::None,
        }
    }
}
impl<E> Page<E> {
    /// The last successful facts, including during a refresh.
    #[must_use]
    pub const fn inventory(&self) -> Option<&Inventory> {
        self.inventory.as_ref()
    }
    /// Initial discovery has no rows to retain. A cached refresh instead
    /// keeps its rows and their typed activity markers.
    #[must_use]
    pub fn initial_discovery(&self) -> bool {
        self.inventory.is_none()
            && self
                .pending
                .as_ref()
                .is_some_and(|ticket| ticket.operation == Operation::Discovery)
    }
    /// The agent whose details are open on this machine.
    #[must_use]
    pub const fn selected(&self) -> Option<&HarnessRef> {
        self.selected.as_ref()
    }
    /// The action still in flight.
    #[must_use]
    pub const fn pending(&self) -> Option<&Ticket> {
        self.pending.as_ref()
    }
    /// An error with the action that produced it.
    #[must_use]
    pub const fn issue(&self) -> Option<&Issue<E>> {
        self.issue.as_ref()
    }
    /// The job and its bounded log, independent of a detail selection.
    #[must_use]
    pub const fn install(&self) -> &InstallProgress {
        &self.install
    }
}

/// Refusal before a request is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StartProblem {
    /// One request on this machine is still running.
    #[error("a request on this machine is still running")]
    Busy,
    /// Every cached page is active and must remain reachable.
    #[error("all Settings request slots are active")]
    Capacity,
    /// Never wrap and accidentally accept a stale response.
    #[error("Settings request ids are exhausted")]
    Exhausted,
}

impl StartProblem {
    /// The refusal remains beside the action; exhaustion needs a restart.
    #[must_use]
    pub const fn variant(self) -> crate::alert::Variant {
        match self {
            Self::Busy | Self::Capacity => crate::alert::Variant::Warning,
            Self::Exhausted => crate::alert::Variant::Error,
        }
    }
}

/// Bounded, request-correlated machine pages.
#[derive(Debug)]
pub struct Pages<E> {
    next: RequestId,
    entries: BTreeMap<MachineKey, Page<E>>,
}
impl<E> Default for Pages<E> {
    fn default() -> Self {
        Self {
            next: RequestId::ZERO,
            entries: BTreeMap::new(),
        }
    }
}

impl<E> Pages<E> {
    /// A page keyed by the frame's existing book name.
    #[must_use]
    pub fn get(&self, machine: &str) -> Option<&Page<E>> {
        self.entries.get(machine)
    }
    /// Open or close an agent's details without changing any in-flight request.
    pub fn select(&mut self, machine: &str, agent: Option<HarnessRef>) {
        if let Some(page) = self.entries.get_mut(machine) {
            page.selected = agent;
        }
    }

    /// Start a request, retaining previous facts but clearing that action's error.
    ///
    /// # Errors
    /// [`StartProblem`] when busy, full or out of correlation ids.
    pub fn begin(
        &mut self,
        machine: MachineKey,
        operation: Operation,
    ) -> Result<RequestId, StartProblem> {
        if self
            .entries
            .get(machine.as_str())
            .is_some_and(|page| page.pending.is_some())
        {
            return Err(StartProblem::Busy);
        }
        let id = self.next.next().ok_or(StartProblem::Exhausted)?;
        if !self.entries.contains_key(machine.as_str()) && self.entries.len() >= MAX_PAGES {
            let idle = self
                .entries
                .iter()
                .find(|(_, page)| page.pending.is_none())
                .map(|(key, _)| key.clone())
                .ok_or(StartProblem::Capacity)?;
            let _gone = self.entries.remove(&idle);
        }
        self.next = id;
        let page = self.entries.entry(machine).or_default();
        page.issue = None;
        if matches!(operation, Operation::Install(_)) {
            page.install = InstallProgress::None;
        }
        page.pending = Some(Ticket { id, operation });
        Ok(id)
    }

    fn pending(&mut self, machine: &MachineKey, id: RequestId) -> Option<&mut Page<E>> {
        self.entries
            .get_mut(machine.as_str())
            .filter(|page| page.pending.as_ref().is_some_and(|ticket| ticket.id == id))
    }
}

mod transitions;

#[cfg(test)]
mod tests;
