//! A validated record-only form travels through the existing bounded Hub once.
use vessel_app_view::agent_status::{addition::AdditionRequest, pages::Operation};
use vessel_proto::{NodeId, agents::AddAgent};

/// Link-owned call; the drawing code cannot mint an unchecked target.
#[derive(Debug)]
pub struct Addition {
    node: NodeId,
    call: AddAgent,
}
impl Addition {
    pub(super) fn operation(&self) -> Operation {
        Operation::Add {
            node: self.node,
            call: Box::new(self.call.clone()),
        }
    }
}
impl crate::Command {
    /// Consume a validated form's route, node, call and existing page ticket.
    #[must_use]
    pub fn add_agent(request: AdditionRequest) -> Self {
        let (machine, node, call, request) = request.into_parts();
        Self::Agents {
            machine,
            request,
            ask: super::Ask::Add(Addition { node, call }),
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::AdditionError;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use native::add;
