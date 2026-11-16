//! Connecting to a node on this machine through its local socket (ADR
//! 0007): a Unix socket, or a named pipe on Windows. The first frame says
//! who connects: the node's token, or a session agent's credential (ADR
//! 0013, `AgentAuth`). Then the protocol runs as on any transport.
//!
//! [Decision Log]
//! - Purpose: `vessel mcp` runs inside a session and reaches its own node
//!   with its session's credential, which only the local socket accepts.
//! - Alternatives: the websocket with the token (the agent would hold the
//!   machine owner's authority, and a node may have no TCP port); going
//!   through `vesseld attach` (a process per connection, and it presents
//!   the token).
//! - Choice: open the endpoint the node names in the `vessel` server's
//!   environment, send the credential frame, and hand the framed stream to
//!   [`NodeClient::over`]. A refused credential is the handshake's
//!   `Rejected` (`unauthorized`).
//! - The Windows pipe's busy retry repeats `vessel-node`'s (`ipc`), which
//!   this crate cannot depend on.

use std::io;
use std::path::{Path, PathBuf};

use futures_util::SinkExt as _;
use tokio::io::AsyncWriteExt as _;
use vessel_proto::{AgentAuth, ClientInfo};
use vessel_rpc::{ClientConfig, Framed, FramedError, MAX_FRAME};

use crate::client::NodeClient;
use crate::token::BearerToken;

/// Who connects on the local socket.
#[derive(Debug, Clone)]
pub enum LocalCredential {
    /// The node's token: the machine's owner.
    Token(BearerToken),
    /// A session's agent (ADR 0013).
    Agent(AgentAuth),
}

/// Why a local connection failed.
#[derive(Debug, thiserror::Error)]
pub enum LocalConnectError {
    /// An explicitly opened transport failed, without retaining its private name.
    #[error("cannot open the local transport")]
    Open(#[source] io::Error),
    /// Kernel process evidence could not be queried on the opened transport.
    #[error("cannot qualify the local peer process")]
    Peer(#[source] io::Error),
    /// The one private pre-auth challenge could not be written.
    #[error("cannot write the private launch challenge")]
    Bootstrap(#[source] io::Error),
    /// No node answers at the endpoint.
    #[error("cannot reach the node's local socket {}", path.display())]
    Unreachable {
        /// The socket path or pipe name.
        path: PathBuf,
        /// Why.
        #[source]
        source: io::Error,
    },
    /// The credential could not be encoded or sent.
    #[error("cannot present the credential")]
    Present(#[source] FramedError),
    /// The credential could not be encoded.
    #[error("cannot encode the credential")]
    Encode(#[source] serde_json::Error),
    /// The protocol handshake failed; a refused credential is `Rejected`
    /// with `unauthorized`.
    #[error("the handshake failed")]
    Handshake(#[source] vessel_rpc::ConnectError<FramedError>),
}

/// One opened, unauthenticated local connection, with no reconnect operation.
///
/// Owning launchers qualify this retained peer before calling authenticate.
/// Opening alone grants no process authority. Canonical Attached callers keep
/// their existing token-first policy through `NodeClient::connect_local`.
#[derive(Debug)]
pub struct LocalConnection {
    transport: vessel_rpc::LocalTransport,
}

impl LocalConnection {
    /// Kernel process evidence from this handle; missing evidence is not proof.
    ///
    /// # Errors
    /// Unsupported platform, failed kernel query or invalid process identity.
    pub fn peer_process(&self) -> Result<Option<std::num::NonZeroU32>, LocalConnectError> {
        self.transport
            .peer_process()
            .map_err(LocalConnectError::Peer)
    }

    /// Write exactly one fresh private challenge before any credential.
    ///
    /// Use only with an explicitly launched private bootstrap endpoint. The
    /// canonical endpoint has no challenge path. A successful write is not
    /// proof: the caller still verifies inherited stdout from its retained child.
    ///
    /// # Errors
    /// The original handle's write fails; this method never reopens it.
    pub async fn send_launch_challenge(
        &mut self,
        challenge: vessel_proto::local_launch::BootstrapChallenge,
    ) -> Result<(), LocalConnectError> {
        self.transport
            .write_all(challenge.encode().as_bytes())
            .await
            .map_err(LocalConnectError::Bootstrap)
    }

    /// Consume this SAME preopened handle into authentication and RPC serving.
    ///
    /// Owning launchers must finish their peer proof first. Closure is terminal;
    /// neither authentication nor `NodeClient` transparently redials an endpoint.
    ///
    /// # Errors
    /// Credential encoding/output or protocol handshake failure.
    pub async fn authenticate(
        self,
        credential: &LocalCredential,
        client: ClientInfo,
    ) -> Result<NodeClient, LocalConnectError> {
        let (reader, writer) = tokio::io::split(self.transport);
        let mut framed = Framed::new(reader, writer, MAX_FRAME);
        let first = match credential {
            LocalCredential::Token(token) => token.expose().to_owned(),
            LocalCredential::Agent(auth) => {
                serde_json::to_string(auth).map_err(LocalConnectError::Encode)?
            }
        };
        framed
            .send(first)
            .await
            .map_err(LocalConnectError::Present)?;
        NodeClient::over(framed, client, &ClientConfig::default())
            .await
            .map_err(LocalConnectError::Handshake)
    }
}

impl NodeClient {
    /// Open a local handle without emitting any credential or Hello bytes.
    ///
    /// The endpoint is not retained in errors, so launch-private names stay out
    /// of logs. Ownership still requires the caller's qualified peer proof.
    ///
    /// # Errors
    /// The platform cannot open the local handle.
    pub async fn open_local(endpoint: &Path) -> Result<LocalConnection, LocalConnectError> {
        Ok(LocalConnection {
            transport: vessel_rpc::LocalTransport::open(endpoint)
                .await
                .map_err(LocalConnectError::Open)?,
        })
    }

    /// Connect to the node's canonical local endpoint, then present credential.
    ///
    /// This retains the existing Attached policy. It never grants child authority.
    ///
    /// # Errors
    /// Local connection, credential output or protocol handshake failure.
    pub async fn connect_local(
        endpoint: &Path,
        credential: &LocalCredential,
        client: ClientInfo,
    ) -> Result<Self, LocalConnectError> {
        let transport = vessel_rpc::LocalTransport::open(endpoint)
            .await
            .map_err(|source| LocalConnectError::Unreachable {
                path: endpoint.to_owned(),
                source,
            })?;
        LocalConnection { transport }
            .authenticate(credential, client)
            .await
    }
}

#[cfg(test)]
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod tests;
