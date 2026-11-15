//! Record-only provider intent, correlated with the existing machine page.
//!
//! [Decision Log]
//! - Purpose: adding a registry command must not also run its package runner.
//! - Constraints: the node owns platform resolution and the shared agent/profile
//!   namespace; a book name is not physical node identity.
//! - Choice: validate the original form against the current inventory, reserve
//!   one pending ticket and accept only the matching authoritative Add ACK.
//! - Alternatives: replace=true, truncate registry ids, or discover after Add.
//! - Impact: no command is started here; uncertain mutations require a fresh read.
use super::{
    catalogue::{self, Offer, Read},
    pages::{Administration, MachineKey, Operation, Pages, RemovalFreshness, StartProblem},
};
use vessel_proto::{
    HarnessRef, NodeId, RequestId,
    agents::{
        AddAgent, AgentDefinition, AgentDetail, AgentInfo, AgentSource, AgentToAdd, CatalogueEntry,
    },
};

/// Original form identity, independent from the current selection. Not cloneable.
#[derive(Debug)]
pub struct Intent {
    machine: MachineKey,
    node: NodeId,
    source: Source,
}
#[derive(Debug)]
enum Source {
    Command,
    Registry(Box<CatalogueEntry>),
}

/// No administrative frame was sent for these validation failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    /// Node's shared name parser rejected it, including reserved builtin names.
    #[error("choose a valid provider name")]
    Name(#[source] vessel_proto::agents::AgentNameError),
    /// Replacing an existing definition was never authorized.
    #[error("a provider already has this name")]
    Taken,
    /// The program is validated as a program, not parsed as a shell command.
    #[error("enter a valid provider program")]
    Program(#[source] vessel_proto::agents::InvalidAgentText),
    /// The original route no longer selects this form's machine.
    #[error("the selected machine changed")]
    MachineChanged,
    /// A fresh connection is needed before preparing an administrative action.
    #[error("provider definitions are unavailable or must be read again")]
    Unavailable,
    /// The physical node or the pinned registry entry changed.
    #[error("the machine or registry entry changed")]
    Changed,
    /// Existing machine-page admission refused another request.
    #[error(transparent)]
    Request(#[from] StartProblem),
}
/// A validated call and its already reserved page ticket. No public constructor.
#[derive(Debug)]
#[must_use = "dispatch this addition or settle its pending ticket"]
pub struct AdditionRequest {
    machine: MachineKey,
    node: NodeId,
    call: AddAgent,
    ticket: RequestId,
}
impl AdditionRequest {
    /// Existing page identity, for the form's pending/ACK presentation.
    #[must_use]
    pub const fn ticket(&self) -> RequestId {
        self.ticket
    }

    /// Route and call are inseparable after consuming this intent.
    #[must_use]
    pub fn into_parts(self) -> (MachineKey, NodeId, AddAgent, RequestId) {
        (self.machine, self.node, self.call, self.ticket)
    }
}
impl Intent {
    /// Open a command form from current, administrable machine facts.
    /// # Errors
    /// [`Refused`] when current definitions do not authorize a new form.
    pub fn command<E>(pages: &Pages<E>, machine: MachineKey) -> Result<Self, Refused> {
        let node = inventory(pages, &machine)?.node;
        Ok(Self {
            machine,
            node,
            source: Source::Command,
        })
    }
    /// Retain the exact entry that a person chose, including its pinned version.
    /// # Errors
    /// [`Refused`] when the node does not offer a record-only command.
    pub fn registry<E>(
        pages: &Pages<E>,
        machine: MachineKey,
        entry: &CatalogueEntry,
    ) -> Result<Self, Refused> {
        let node = inventory(pages, &machine)?.node;
        check_entry(pages, &machine, entry)?;
        Ok(Self {
            machine,
            node,
            source: Source::Registry(Box::new(entry.clone())),
        })
    }
    /// Validate the whole form, reserving no ticket on refusal. Command arguments
    /// are one argument per line; no shell splitting or quote interpretation.
    /// # Errors
    /// [`Refused`] on changed intent, invalid input or existing page activity.
    pub fn submit<E>(
        &self,
        shown: &MachineKey,
        pages: &mut Pages<E>,
        name: &str,
        program: &str,
        arguments: &str,
    ) -> Result<AdditionRequest, Refused> {
        if shown != &self.machine {
            return Err(Refused::MachineChanged);
        }
        let current = inventory(pages, &self.machine)?;
        if current.node != self.node {
            return Err(Refused::Changed);
        }
        let name = name
            .parse::<vessel_proto::agents::AgentName>()
            .map_err(Refused::Name)?;
        if current.agents.iter().any(|info| match &info.agent {
            HarnessRef::Added(existing) => existing.as_str().eq_ignore_ascii_case(name.as_str()),
            HarnessRef::Known(_) => false,
        }) {
            return Err(Refused::Taken);
        }
        let agent = match &self.source {
            Source::Command => AgentToAdd::Command {
                definition: AgentDefinition {
                    label: None,
                    program: program.parse().map_err(Refused::Program)?,
                    args: arguments.lines().map(str::to_owned).collect(),
                    env: Vec::new(),
                    inherit: Vec::new(),
                    mcp: Vec::new(),
                },
            },
            Source::Registry(entry) => {
                check_entry(pages, &self.machine, entry)?;
                AgentToAdd::Registry {
                    id: entry.id.clone(),
                    version: entry.version.clone(),
                    mcp: Vec::new(),
                }
            }
        };
        let call = AddAgent {
            name,
            agent,
            replace: false,
        };
        let ticket = pages.begin(
            self.machine.clone(),
            Operation::Add {
                node: self.node,
                call: Box::new(call.clone()),
            },
        )?;
        Ok(AdditionRequest {
            machine: self.machine.clone(),
            node: self.node,
            call,
            ticket,
        })
    }
}
fn inventory<'a, E>(
    pages: &'a Pages<E>,
    machine: &MachineKey,
) -> Result<&'a super::pages::Inventory, Refused> {
    let page = pages.get(machine.as_str()).ok_or(Refused::Unavailable)?;
    if page.pending().is_some() {
        return Err(StartProblem::Busy.into());
    }
    if page.removal_freshness() != RemovalFreshness::Current {
        return Err(Refused::Unavailable);
    }
    page.inventory()
        .filter(|value| value.administration == Administration::Supported)
        .ok_or(Refused::Unavailable)
}
fn check_entry<E>(
    pages: &Pages<E>,
    machine: &MachineKey,
    entry: &CatalogueEntry,
) -> Result<(), Refused> {
    let current = inventory(pages, machine)?;
    let copy = match pages.get(machine.as_str()).map(|page| page.catalogue()) {
        Some(
            Read::Ready(copy)
            | Read::Failed {
                previous: Some(copy),
                ..
            },
        ) => copy,
        Some(Read::NotRequested | Read::Pending { .. } | Read::Failed { previous: None, .. })
        | None => return Err(Refused::Unavailable),
    };
    if copy.fetched_at.is_none() {
        return Err(Refused::Unavailable);
    }
    if !copy.entries.iter().any(|present| present == entry)
        || !matches!(catalogue::offer(entry, &current.agents), Offer::Add(_))
    {
        return Err(Refused::Changed);
    }
    Ok(())
}
/// Accept only a definition with this exact requested name and source.
#[must_use]
pub fn matches_ack(call: &AddAgent, info: &AgentInfo) -> bool {
    if info.agent != HarnessRef::Added(call.name.clone()) {
        return false;
    }
    let AgentDetail::Added { added } = &info.detail else {
        return false;
    };
    match (&call.agent, &added.source) {
        (AgentToAdd::Command { .. }, AgentSource::Command) => true,
        (
            AgentToAdd::Registry { id, version, .. },
            AgentSource::Registry {
                id: actual_id,
                version: actual_version,
            },
        ) => id == actual_id && version == actual_version,
        (AgentToAdd::Command { .. }, AgentSource::Registry { .. })
        | (AgentToAdd::Registry { .. }, AgentSource::Command) => false,
    }
}
#[cfg(test)]
mod tests;

/// Whole instructions for a dispatched addition, distinct from form validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// No frame reached the different physical machine.
    Changed,
    /// The typed client or node refused an unsupported method.
    Unsupported,
    /// This build has no assigned method; updating the peer cannot fix it.
    BuildUnavailable,
    /// Local request identities cannot be safely reused in this client.
    Exhausted,
    /// Node Admin/local-person policy refused the action.
    Permission,
    /// Agent/profile name or pinned registry version conflicts.
    Conflict,
    /// The server rejected the supplied definition.
    Rejected,
    /// The file may have changed; refresh before any new decision.
    Unknown,
    /// No client was available before dispatch.
    Offline,
    /// Existing Hub admission refused a worker before dispatch.
    Busy,
    /// Browser transport is not implemented.
    NoTransport,
}
impl Failure {
    /// Uncertainty is caution; definite refusals are failures.
    #[must_use]
    pub const fn variant(self) -> crate::alert::Variant {
        match self {
            Self::Unknown | Self::Busy => crate::alert::Variant::Warning,
            Self::Changed
            | Self::Unsupported
            | Self::BuildUnavailable
            | Self::Exhausted
            | Self::Permission
            | Self::Conflict
            | Self::Rejected
            | Self::Offline
            | Self::NoTransport => crate::alert::Variant::Error,
        }
    }
}

/// A default name is offered only when the whole id satisfies the node rule.
/// Longer or reserved registry ids stay editable/blank, never truncated.
#[must_use]
pub fn suggested_name(entry: &CatalogueEntry) -> Option<vessel_proto::agents::AgentName> {
    entry.id.as_str().parse().ok()
}
