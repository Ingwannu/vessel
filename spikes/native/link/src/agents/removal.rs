//! A removal travels through the existing bounded Agents hub once.
//!
//! [Decision Log]
//! - Purpose: a confirmed book route may reconnect to another node before I/O.
//! - Constraints: the node accepts a name, no revision precondition; loss of an
//!   acknowledgement does not prove that its durable write was cancelled.
//! - Alternatives: repeat on reconnect or interpret a timeout as non-removal.
//! - Choice: consume the view request, compare current connection identity,
//!   bound the one call and retain typed refusal/uncertainty for the page.
//! - Cost: an uncertain result requires discovery before fresh confirmation.

use vessel_app_view::agent_status::{pages::Operation, removal::RemovalRequest};
use vessel_proto::{NodeId, agents::AgentName};

/// A non-cloneable target, constructed only from a consumed confirmation.
#[derive(Debug)]
pub struct Removal {
    node: NodeId,
    name: AgentName,
}

impl Removal {
    pub(super) fn operation(&self) -> Operation {
        Operation::Remove {
            node: self.node,
            name: self.name.clone(),
        }
    }
}

impl crate::Command {
    /// Consume a confirmed page ticket; the route, target and reply identity
    /// cannot be substituted independently by the drawing code.
    #[must_use]
    pub fn remove_agent(request: RemovalRequest) -> Self {
        let (machine, node, name, request) = request.into_parts();
        Self::Agents {
            machine,
            request,
            ask: super::Ask::Remove(Removal { node, name }),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::RemovalError;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use native::remove;
