//! [`NodeConnector`]: how the phone reaches one book entry.
//!
//! [Decision Log]
//! - Purpose: the bridge dials each node through its source's connector,
//!   again after every loss.
//! - Constraint: a phone has no `ssh` to start and no node of its own; a
//!   paired node (direct routes or relays, end to end) is what it uses,
//!   and a websocket node with a token file is kept for a book copied
//!   from a desktop.
//! - Alternatives: depending on `vessel-cli` for its connector (a binary
//!   crate, with `vessel-remote`, `vessel-node` and SSH behind it); moving
//!   the CLI's connector into `vessel-client` now (the right home if this
//!   shell is kept, but a change to a crate other tracks hold, for a
//!   spike).
//! - Choice: the CLI's enum without its SSH arm, written again here (60
//!   lines); every dial still goes through `Pinned`, so only the node the
//!   book pinned is accepted.
//! - Cost: the two copies can drift until they are joined in
//!   `vessel-client`, which this spike's notes recommend.

use vessel_client::secure::{DialError, ReachError, SecureConnector, WsDialer};
use vessel_client::{ConnectError, Connector, NodeClient, Pinned, PinnedError, WsConnector};
use vessel_proto::ClientInfo;

/// Opens connections to one node, whatever its transport.
#[derive(Debug)]
pub enum NodeConnector {
    /// A websocket and a token.
    Ws(Pinned<WsConnector>),
    /// A paired node over its routes (boxed: several times the size of a
    /// websocket connector).
    Secure(Box<Pinned<SecureConnector<WsDialer>>>),
}

/// Why [`NodeConnector`] could not connect.
#[derive(Debug, thiserror::Error)]
pub enum NodeConnectError {
    /// The websocket node did not answer, or another node did.
    #[error(transparent)]
    Ws(PinnedError<ConnectError>),
    /// No route reached the paired node, or another node answered.
    #[error(transparent)]
    Secure(PinnedError<ReachError<DialError>>),
}

impl Connector for NodeConnector {
    type Error = NodeConnectError;

    async fn connect(&self, client: ClientInfo) -> Result<NodeClient, NodeConnectError> {
        match self {
            Self::Ws(connector) => connector
                .connect(client)
                .await
                .map_err(NodeConnectError::Ws),
            Self::Secure(connector) => connector
                .connect(client)
                .await
                .map_err(NodeConnectError::Secure),
        }
    }

    fn is_transient(&self, error: &NodeConnectError) -> bool {
        match (self, error) {
            (Self::Ws(connector), NodeConnectError::Ws(error)) => connector.is_transient(error),
            (Self::Secure(connector), NodeConnectError::Secure(error)) => {
                connector.is_transient(error)
            }
            // A connector only returns its own errors.
            (Self::Ws(_), NodeConnectError::Secure(_))
            | (Self::Secure(_), NodeConnectError::Ws(_)) => false,
        }
    }
}
