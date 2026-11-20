//! Binding the local socket and the TCP address, before any task starts.

use std::net::SocketAddr;

use tokio::net::TcpListener;
use vessel_proto::{NodeId, RelayToken};
use vessel_relay::address::ClientAddress;
use vessel_relay::TlsConfig;
use vessel_store::DataDir;

use crate::config::Config;
use crate::error::NodeError;
use crate::ipc::{Endpoint, LocalListener};
use crate::relays;

/// Bind the local socket, and the TCP address if the config has one.
pub(super) async fn bind(
    config: &Config,
    id: NodeId,
) -> Result<(LocalListener, Option<(TcpListener, SocketAddr)>), NodeError> {
    let endpoint = Endpoint::of(&config.data_dir, id);
    let path = endpoint.path().to_path_buf();
    let local =
        LocalListener::bind(endpoint).map_err(|source| NodeError::LocalSocket { path, source })?;
    let tcp = match config.bind {
        Some(bind) => Some(listen(bind, &config.data_dir).await?),
        None => None,
    };
    Ok((local, tcp))
}

/// Where a direct client counts: behind a trusted reverse proxy, by its
/// header (`--client-address-header`, `--trusted-proxy`), as a relay does.
pub(super) fn client_address(config: &Config) -> Result<ClientAddress, NodeError> {
    let clients = ClientAddress::new(
        config.client_address_header.clone(),
        config.trusted_proxies.clone(),
    )?;
    if config.client_address_header.is_none() && !config.trusted_proxies.is_empty() {
        tracing::warn!("trusted proxies are ignored without a client address header");
    }
    Ok(clients)
}

/// The relays' registration token and the TLS setup for them, if any
/// relay is configured. The token file is read even without relays, so a
/// broken one shows before it is needed.
pub(super) fn relaying(config: &Config) -> Result<Option<(RelayToken, TlsConfig)>, NodeError> {
    let token = match &config.relay_token_file {
        Some(path) => Some(vessel_relay::read_token_file(path)?),
        None => None,
    };
    if config.relays.is_empty() {
        return Ok(None);
    }
    let token = token.ok_or(NodeError::NoRelayToken)?;
    Ok(Some((token, relays::relay_tls(&config.relays)?)))
}

/// Bind the TCP listen address.
pub(super) async fn listen(
    bind: SocketAddr,
    dir: &DataDir,
) -> Result<(TcpListener, SocketAddr), NodeError> {
    let listener = TcpListener::bind(bind)
        .await
        .map_err(|source| NodeError::Bind {
            address: bind,
            source,
        })?;
    let address = listener.local_addr().map_err(|source| NodeError::Bind {
        address: bind,
        source,
    })?;
    if !address.ip().is_loopback() {
        tracing::warn!(
            %address,
            token = %dir.token_path().display(),
            "listening beyond loopback: anyone who can reach this address and \
             read the token file can drive this node's harnesses"
        );
    }
    Ok((listener, address))
}
