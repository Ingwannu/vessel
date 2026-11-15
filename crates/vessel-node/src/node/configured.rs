//! Startup relay authority comes only from operator configuration.
//!
//! Keep token/TLS preparation and the shared configured owner together. Settings
//! can toggle consent later, but cannot introduce routes, credentials or TLS.
use crate::{
    config::Config,
    error::NodeError,
    fleet::HomeRelays,
    relays::{self, Relays},
};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use vessel_proto::RelayToken;
use vessel_relay::TlsConfig;
use vessel_secure::NodeKeypair;

pub(super) struct Startup {
    pub(super) relays: Arc<Relays>,
    pub(super) home: Option<HomeRelays>,
}

/// Reject a configured count before token/TLS I/O or any registration task.
/// Preserve the old token-file check even when no routes are configured.
pub(super) fn recipe(config: &Config) -> Result<Option<(RelayToken, TlsConfig)>, NodeError> {
    if config.relays.len() > relays::MAX_CONFIGURED_RELAYS {
        return Err(NodeError::TooManyConfiguredRelays);
    }
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

pub(super) fn start(
    config: &Config,
    keypair: &Arc<NodeKeypair>,
    stop: &CancellationToken,
    recipe: Option<(RelayToken, TlsConfig)>,
) -> Result<Startup, NodeError> {
    let relays = Arc::new(Relays::new(Arc::clone(keypair), stop.clone()));
    let home = match recipe {
        None => None,
        Some((token, tls)) => {
            relays.link_configured(&config.relays, &token, &tls)?;
            // Fleet callback tickets keep their existing independent authority.
            Some(HomeRelays {
                relays: config.relays.clone(),
                token,
                first_window: config.fleet.first_ticket_window,
            })
        }
    };
    Ok(Startup { relays, home })
}
