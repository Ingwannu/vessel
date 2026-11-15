//! Inbox retains only the reviewed selected-call boundary. Metadata is copied
//! from that original admission; effects always pass its before/after fence.
use crate::connections::{RequestFailure, SelectedConnection};
use vessel_app_view::connection::ConnectionContext;
use vessel_proto::{NodeId, ProtocolVersion};

#[derive(Clone)]
pub(super) struct Client {
    selected: SelectedConnection,
    context: ConnectionContext,
    protocol: ProtocolVersion,
}
impl Client {
    pub(super) const fn new(
        selected: SelectedConnection,
        context: ConnectionContext,
        protocol: ProtocolVersion,
    ) -> Self {
        Self {
            selected,
            context,
            protocol,
        }
    }
    pub(super) fn node(&self) -> NodeId {
        self.context.node()
    }
    pub(super) const fn protocol(&self) -> ProtocolVersion {
        self.protocol
    }
    pub(super) async fn call<R: vessel_client::Unary>(
        &self,
        request: R,
    ) -> Result<R::Response, RequestFailure> {
        self.selected.call(request).await
    }
}
// A test or decoder may already have a typed SDK failure. Both go through the
// same closed mapper; RequestFailure's Debug/source chain stay redacted.
impl From<vessel_client::CallError> for RequestFailure {
    fn from(error: vessel_client::CallError) -> Self {
        Self::Call(error)
    }
}
