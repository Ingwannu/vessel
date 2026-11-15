//! Retain the exact fleet destination and origin across related transfer calls.
//! Authorization stays at the envelope and receiving guard; this is transport only.

use crate::fleet::Links;
use std::sync::Arc;
use vessel_proto::{
    Call, CloseTransfer, Deliver, ErrorCode, Forwarded, NodeId, Origin, RelayCall, Request as _,
    TransferId, WireError,
};

#[derive(Debug, Clone, Copy)]
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
    pub(super) fn forward(
        links: &Arc<Links>,
        node: NodeId,
        origin: Origin,
    ) -> Result<Self, WireError> {
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
        Ok(Self {
            links: Arc::clone(links),
            node,
            origin,
            hop,
        })
    }

    pub(super) fn member(links: &Arc<Links>, node: NodeId, origin: Origin) -> Self {
        Self {
            links: Arc::clone(links),
            node,
            origin,
            hop: Hop::Member,
        }
    }

    pub(super) async fn call(&self, call: Box<Call>) -> Result<Forwarded, WireError> {
        match frame(self.hop, self.node, &self.origin, call) {
            Frame::Member { node, request } => self.links.call_member(node, request).await,
            Frame::Home(request) => self.links.call_home(request).await,
        }
    }

    pub(super) async fn close(self, transfer: TransferId) -> Result<(), WireError> {
        let forwarded = self
            .call(Box::new(CloseTransfer { transfer }.into_call()))
            .await?;
        CloseTransfer::response_from(*forwarded.reply)
            .map(drop)
            .map_err(|unexpected| {
                WireError::new(
                    ErrorCode::Internal,
                    format!("node {} answered out of turn: {unexpected}", self.node),
                )
            })
    }
}

// A member frame deliberately leaves direct versus callback transport to Links;
// cleanup must not replace that policy with a new dial or a different principal.
enum Frame {
    Member { node: NodeId, request: Deliver },
    Home(RelayCall),
}

fn frame(hop: Hop, node: NodeId, origin: &Origin, call: Box<Call>) -> Frame {
    let origin = origin.clone();
    match hop {
        Hop::Member => Frame::Member {
            node,
            request: Deliver { call, origin },
        },
        Hop::Home => Frame::Home(RelayCall { node, call, origin }),
    }
}

#[cfg(test)]
mod tests;
