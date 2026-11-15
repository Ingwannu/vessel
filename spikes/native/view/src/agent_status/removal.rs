//! An added provider is removed only after confirmation of the same definition.
//!
//! [Decision Log]
//! - Purpose: a dialog opened for one machine must not delete from another,
//!   and a refreshed definition must not silently replace the confirmed one.
//! - Constraint: `agent.remove` accepts a name, with no revision precondition.
//! - Alternatives: let a callback retain only the visible row index or name.
//! - Choice: retain the typed machine, name and definition until confirmation;
//!   consume that confirmation once and use the page's existing request ticket.
//! - Cost: changes by another client after dispatch remain the node's policy;
//!   this snapshot prevents stale local intent, not a distributed transaction.

use super::pages::{Administration, Inventory, MachineKey, Operation, Pages, StartProblem};
use vessel_proto::agents::{AddedAgent, AgentDetail, AgentName};
use vessel_proto::{HarnessRef, NodeId, RequestId};

/// Whether the node actually lists an added definition that it can administer.
#[must_use]
pub fn can_remove(agent: &HarnessRef, inventory: &Inventory) -> bool {
    definition(agent, inventory).is_some()
}

fn definition<'a>(agent: &HarnessRef, inventory: &'a Inventory) -> Option<&'a AddedAgent> {
    match (&inventory.administration, agent) {
        (Administration::Supported, HarnessRef::Added(_)) => inventory
            .agents
            .iter()
            .find(|info| info.agent == *agent)
            .and_then(|info| match &info.detail {
                AgentDetail::Added { added } => Some(added),
                AgentDetail::Known { .. } => None,
            }),
        (Administration::Supported, HarnessRef::Known(_))
        | (Administration::OlderNode, HarnessRef::Known(_) | HarnessRef::Added(_)) => None,
    }
}

/// A confirmation's target stays independent of the current row selection.
/// Not cloneable: confirming consumes the person's one decision.
#[derive(Debug)]
pub struct Confirmation {
    machine: MachineKey,
    node: NodeId,
    name: AgentName,
    definition: AddedAgent,
}

/// A refusal before deletion is sent; no node or file changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    /// A builtin, an unlisted definition, or an older node cannot be removed here.
    #[error("this provider is not an added definition on this machine")]
    Unavailable,
    /// A queued confirmation still belongs to its original machine.
    #[error("the selected machine changed before confirmation")]
    MachineChanged,
    /// A book name now points to a different node.
    #[error("the node behind this machine name changed before confirmation")]
    NodeChanged,
    /// The name now identifies different local facts; ask the person again.
    #[error("the provider definition changed before confirmation")]
    DefinitionChanged,
    /// The existing page request policy refused the operation.
    #[error(transparent)]
    Request(#[from] StartProblem),
}

/// Dispatch identity minted by a consumed confirmation, never a row index.
#[derive(Debug)]
#[must_use = "dispatch this removal or settle its pending ticket"]
pub struct RemovalRequest {
    machine: MachineKey,
    node: NodeId,
    name: AgentName,
    ticket: RequestId,
}

impl RemovalRequest {
    /// Give the link its book route, expected node, added name and reply ticket.
    /// Private fields keep callers from minting unconfirmed dispatch identities.
    #[must_use]
    pub fn into_parts(self) -> (MachineKey, NodeId, AgentName, RequestId) {
        (self.machine, self.node, self.name, self.ticket)
    }
}

impl Confirmation {
    /// Prepare the dialog without starting a request. Dropping it is Cancel.
    ///
    /// # Errors
    /// [`Refused`] if this page is busy or the definition cannot be removed.
    pub fn new<E>(
        pages: &Pages<E>,
        machine: MachineKey,
        agent: &HarnessRef,
    ) -> Result<Self, Refused> {
        let page = pages.get(machine.as_str()).ok_or(Refused::Unavailable)?;
        if page.pending().is_some() {
            return Err(StartProblem::Busy.into());
        }
        let inventory = page.inventory().ok_or(Refused::Unavailable)?;
        let definition = definition(agent, inventory).ok_or(Refused::Unavailable)?;
        let name = match agent {
            HarnessRef::Added(name) => name.clone(),
            HarnessRef::Known(_) => return Err(Refused::Unavailable),
        };
        Ok(Self {
            machine,
            node: inventory.node,
            name,
            definition: definition.clone(),
        })
    }

    /// Recheck intent immediately before dispatch and start its existing ticket.
    ///
    /// # Errors
    /// [`Refused`] when the machine, definition or page state changed.
    pub fn confirm<E>(
        self,
        shown: &MachineKey,
        pages: &mut Pages<E>,
    ) -> Result<RemovalRequest, Refused> {
        if *shown != self.machine {
            return Err(Refused::MachineChanged);
        }
        let agent = HarnessRef::Added(self.name.clone());
        let inventory = pages
            .get(self.machine.as_str())
            .and_then(|page| page.inventory())
            .ok_or(Refused::Unavailable)?;
        if inventory.node != self.node {
            return Err(Refused::NodeChanged);
        }
        let current = definition(&agent, inventory).ok_or(Refused::Unavailable)?;
        if *current != self.definition {
            return Err(Refused::DefinitionChanged);
        }
        let ticket = pages.begin(
            self.machine.clone(),
            Operation::Remove {
                node: self.node,
                name: self.name.clone(),
            },
        )?;
        Ok(RemovalRequest {
            machine: self.machine,
            node: self.node,
            name: self.name,
            ticket,
        })
    }
}

#[cfg(test)]
mod tests;
