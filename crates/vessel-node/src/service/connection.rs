//! Connection identity and the recovery operation attached to one guarded call.

use std::sync::Arc;

use vessel_proto::ProtocolVersion;
use vessel_rpc::{Outbound, Peer};

use crate::connection_life::ConnectionLife;
use crate::principal::Principal;
use crate::waits::ConnectionWaits;

/// Per-connection state: who is calling, and where this connection's
/// subscriptions go.
#[derive(Debug)]
pub(crate) struct Connection {
    operation: Option<Arc<crate::recovery::Operation>>,
    pub(super) outbound: Outbound,
    pub(super) principal: Principal,
    /// The protocol the client negotiated: an older client is never handed
    /// what it cannot decode (protocol 11's added agents).
    pub(super) protocol: ProtocolVersion,
    /// Its waiting reads, against its share (`waits`).
    pub(super) waiting: ConnectionWaits,
    pub(super) presence_id: crate::push::PresenceId,
    /// Its identity and close observer, one per connection
    /// (`connection_life`).
    life: ConnectionLife,
}

impl Connection {
    pub(crate) fn new(peer: Peer, principal: Principal) -> Self {
        tracing::info!(
            client = %peer.client.name,
            version = %peer.client.version,
            ?principal,
            "client connected"
        );
        Self {
            life: ConnectionLife::within(peer.outbound.closing_token()),
            operation: None,
            outbound: peer.outbound,
            principal,
            protocol: peer.protocol,
            waiting: ConnectionWaits::default(),
            presence_id: crate::push::PresenceId::new(),
        }
    }

    /// A connection of the machine's own clients (token or local socket).
    pub(crate) fn owner(peer: Peer) -> Self {
        Self::new(peer, Principal::Owner)
    }

    /// Who is calling.
    pub(crate) const fn principal(&self) -> &Principal {
        &self.principal
    }

    /// The protocol the client speaks.
    pub(crate) const fn protocol(&self) -> ProtocolVersion {
        self.protocol
    }

    /// This connection, calling as `principal`: a call delivered from
    /// another node runs as its delegated principal (protocol 6).
    pub(crate) fn acting_as(&self, principal: Principal) -> Self {
        Self {
            operation: None,
            outbound: self.outbound.clone(),
            principal,
            protocol: self.protocol,
            waiting: Arc::clone(&self.waiting),
            presence_id: self.presence_id,
            life: self.life.clone(),
        }
    }

    /// This connection's identity and close observer, the same for every
    /// context acting for it. A resource the connection owns keeps
    /// `owner()` and `closed()`, never the life: holding the life keeps the
    /// connection's last context from ending it.
    pub(crate) const fn life(&self) -> &ConnectionLife {
        &self.life
    }
}

impl Connection {
    pub(crate) fn with_operation(&self, operation: Arc<crate::recovery::Operation>) -> Self {
        let mut connection = self.acting_as(self.principal.clone());
        connection.operation = Some(operation);
        connection
    }
    pub(crate) fn operation(&self) -> Option<Arc<crate::recovery::Operation>> {
        self.operation.clone()
    }
}
