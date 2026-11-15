//! Retain the exact fleet destination and origin across related transfer calls.
//! Authorization stays at the envelope and receiving guard; this is transport only.

use std::sync::Arc;
use vessel_proto::{Call, Deliver, ErrorCode, Forwarded, NodeId, Origin, RelayCall, WireError};
use crate::fleet::Links;

#[derive(Debug)]
enum Hop {
    Member,
    Home,
}

/// An already authorized envelope's transport path, including callback routing
/// inside Links. Keeping the origin avoids rebuilding a different transfer holder.
#[derive(Debug)]
pub(super) struct Route {
    links: Arc<Links>,
    node: NodeId,
    origin: Origin,
    hop: Hop,
}

impl Route {
    pub(super) fn forward(links: &Arc<Links>, node: NodeId, origin: Origin) -> Result<Self, WireError> {
        let hop = if links.is_home() {
            Hop::Member
        } else if links.is_member() {
            Hop::Home
        } else {
            return Err(WireError::new(
                ErrorCode::NotFound,
                format!("this node is in no fleet, so it cannot reach node {node}"),
            ));
        };
        Ok(Self { links: Arc::clone(links), node, origin, hop })
    }

    pub(super) fn member(links: &Arc<Links>, node: NodeId, origin: Origin) -> Self {
        Self { links: Arc::clone(links), node, origin, hop: Hop::Member }
    }

    pub(super) async fn call(&self, call: Box<Call>) -> Result<Forwarded, WireError> {
        let origin = self.origin.clone();
        match self.hop {
            Hop::Member => self.links.call_member(self.node, Deliver { call, origin }).await,
            Hop::Home => self.links.call_home(RelayCall { node: self.node, call, origin }).await,
        }
    }
}
