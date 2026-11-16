//! Generated peer variables, with raw key text confined to this decode edge.
//! A key is derived from `ScriptName` once; there is no independent normalizer.

use serde::{Deserialize, Serialize};

use crate::{
    scripts::{ScriptName, ServiceEnvName},
    workspace::ServicePort,
};

use super::ServiceProxyUrl;

/// Only generated service fields can enter the cached environment projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "RawServiceEnvPeer")]
pub struct ServiceEnvPeer {
    script: ScriptName,
    env_name: ServiceEnvName,
    port: ServicePort,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<ServiceProxyUrl>,
}

/// Exhaustive projection keeps script and normalized key visible together.
#[derive(Debug, Clone, Copy)]
pub struct ServiceEnvPeerView<'a> {
    /// The approved service's exact name.
    pub script: &'a ScriptName,
    /// Reuse the file parser's normalization owner.
    pub env_name: &'a ServiceEnvName,
    /// A committed plan value, never an observed listening socket.
    pub port: ServicePort,
    /// Missing URL means the generated URL variable is omitted.
    pub url: Option<&'a ServiceProxyUrl>,
}

/// A wire peer cannot substitute another service's environment variable key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidServiceEnvPeer {
    /// Keep submitted raw key text out of the diagnostic.
    #[error("a service environment key must match its script name")]
    Name,
}

impl ServiceEnvPeer {
    /// Derive rather than accept an independently authored normalized key.
    #[must_use]
    pub fn new(script: ScriptName, port: ServicePort, url: Option<ServiceProxyUrl>) -> Self {
        let env_name = ServiceEnvName::of(&script);
        Self {
            script,
            env_name,
            port,
            url,
        }
    }

    /// This descriptive view is never accepted as process launch authority.
    #[must_use]
    pub const fn view(&self) -> ServiceEnvPeerView<'_> {
        let Self {
            script,
            env_name,
            port,
            url,
        } = self;
        ServiceEnvPeerView {
            script,
            env_name,
            port: *port,
            url: url.as_ref(),
        }
    }
}

// The raw string exists only at this boundary, then compares with the one
// existing derivation; it is never stored as a domain value or error cause.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawServiceEnvPeer {
    script: ScriptName,
    env_name: String,
    port: ServicePort,
    #[serde(default)]
    url: Option<ServiceProxyUrl>,
}

impl TryFrom<RawServiceEnvPeer> for ServiceEnvPeer {
    type Error = InvalidServiceEnvPeer;

    fn try_from(raw: RawServiceEnvPeer) -> Result<Self, Self::Error> {
        let peer = Self::new(raw.script, raw.port, raw.url);
        if peer.env_name.as_str() != raw.env_name {
            return Err(InvalidServiceEnvPeer::Name);
        }
        Ok(peer)
    }
}
