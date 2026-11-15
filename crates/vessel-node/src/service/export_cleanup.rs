//! Retain rejected-export cleanup in the node's existing forwarding task lifetime.
//! The transport route is supplied by the envelope, without gaining authority.

use super::NodeService;
use std::future::Future;
use vessel_proto::{Forwarded, ProtocolVersion, TransferId, WireError};

impl NodeService {
    /// Register cleanup before yielding so a disconnected caller cannot lose the
    /// only handle to an export it was refused permission to receive.
    pub(crate) async fn settle_export<F, C>(
        &self,
        value: &Forwarded,
        peer: ProtocolVersion,
        close: C,
    ) -> Result<(), WireError>
    where
        F: Future<Output = Result<(), WireError>> + Send + 'static,
        C: FnOnce(TransferId) -> F + Send + 'static,
    {
        crate::metadata_export::settle(value, peer, &self.forwarders, close)
            .await
            .map_err(|error| error.to_wire())
    }
}

#[cfg(test)]
mod runtime_tests;

#[cfg(test)]
mod envelope_tests;
#[cfg(test)]
mod fixture_peer;
