//! Cached generated fields only; no allocation/helper/env inheritance or launch.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    WorkspaceId,
    scripts::ScriptName,
    workspace::{MAX_PROJECT_COMMANDS, ServicePort},
};

use super::{
    ServiceEnvPeer, ServiceProxyUrl, ServiceSnapshotId, ServiceUnavailable, WorkspaceScriptRef,
};

/// The node RPC bind does not authorize a repository service's network exposure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ServiceBindHost {
    /// The reference's loopback launch host, independent of wildcard RPC bind.
    #[default]
    Ipv4Loopback,
    /// An explicitly selected loopback address in the other address family.
    Ipv6Loopback,
}

impl ServiceBindHost {
    /// Only these generated HOST values exist; no inherited string is accepted.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ipv4Loopback => "127.0.0.1",
            Self::Ipv6Loopback => "::1",
        }
    }
}

/// A projection of an already committed plan, never an executable plan owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "RawReadyServiceEnv")]
pub struct ReadyServiceEnv {
    workspace: WorkspaceId,
    snapshot: ServiceSnapshotId,
    script: ScriptName,
    host: ServiceBindHost,
    port: ServicePort,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<ServiceProxyUrl>,
    peers: Vec<ServiceEnvPeer>,
}

/// Complete readonly display; an added field cannot be silently omitted.
#[derive(Debug, Clone, Copy)]
pub struct ReadyServiceEnvView<'a> {
    /// Canonical identity, not a directory match.
    pub workspace: WorkspaceId,
    /// Metadata etag, not current process/socket proof.
    pub snapshot: ServiceSnapshotId,
    /// The selected service, which must appear in peers.
    pub script: &'a ScriptName,
    /// Generated bind choice does not sandbox an approved program.
    pub host: ServiceBindHost,
    /// Repeated self/peer fields are checked at construction.
    pub port: ServicePort,
    /// Missing means omit the generated URL variable.
    pub url: Option<&'a ServiceProxyUrl>,
    /// All configured services' committed plans, selected service included.
    pub peers: &'a [ServiceEnvPeer],
}

/// Refuse incomplete or contradictory cached plans without repairing them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidReadyServiceEnv {
    /// Page/env counts share the qualified project admission constant.
    #[error("a service environment has at most {max} peers", max = MAX_PROJECT_COMMANDS)]
    TooMany,
    /// Duplicate script/normalized key would overwrite generated variables.
    #[error("service environment peers repeat a normalized key")]
    Duplicate,
    /// Paseo refuses when the selected service is absent from its peer plan.
    #[error("the selected service is missing from its peer plan")]
    MissingSelf,
    /// The generated self values must agree with the selected peer.
    #[error("the selected service and peer plan disagree")]
    SelfMismatch,
}

impl ReadyServiceEnv {
    /// Keep bounded generated fields consistent, without executing or reserving.
    ///
    /// # Errors
    /// Excess peers, duplicate normalized keys, or missing/inconsistent self.
    pub fn new(
        target: WorkspaceScriptRef,
        snapshot: ServiceSnapshotId,
        host: ServiceBindHost,
        port: ServicePort,
        url: Option<ServiceProxyUrl>,
        peers: Vec<ServiceEnvPeer>,
    ) -> Result<Self, InvalidReadyServiceEnv> {
        if peers.len() > MAX_PROJECT_COMMANDS {
            return Err(InvalidReadyServiceEnv::TooMany);
        }
        let mut seen = HashSet::with_capacity(peers.len());
        for peer in &peers {
            if !seen.insert(peer.view().env_name) {
                return Err(InvalidReadyServiceEnv::Duplicate);
            }
        }
        let selected = peers
            .iter()
            .find(|peer| peer.view().script == &target.script)
            .ok_or(InvalidReadyServiceEnv::MissingSelf)?;
        if selected.view().port != port || selected.view().url != url.as_ref() {
            return Err(InvalidReadyServiceEnv::SelfMismatch);
        }
        Ok(Self {
            workspace: target.workspace,
            snapshot,
            script: target.script,
            host,
            port,
            url,
            peers,
        })
    }

    /// Expose every generated field, with no arbitrary environment map.
    #[must_use]
    pub fn view(&self) -> ReadyServiceEnvView<'_> {
        let Self {
            workspace,
            snapshot,
            script,
            host,
            port,
            url,
            peers,
        } = self;
        ReadyServiceEnvView {
            workspace: *workspace,
            snapshot: *snapshot,
            script,
            host: *host,
            port: *port,
            url: url.as_ref(),
            peers,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReadyServiceEnv {
    workspace: WorkspaceId,
    snapshot: ServiceSnapshotId,
    script: ScriptName,
    host: ServiceBindHost,
    port: ServicePort,
    #[serde(default)]
    url: Option<ServiceProxyUrl>,
    peers: Vec<ServiceEnvPeer>,
}

impl TryFrom<RawReadyServiceEnv> for ReadyServiceEnv {
    type Error = InvalidReadyServiceEnv;

    fn try_from(raw: RawReadyServiceEnv) -> Result<Self, Self::Error> {
        Self::new(
            WorkspaceScriptRef {
                workspace: raw.workspace,
                script: raw.script,
            },
            raw.snapshot,
            raw.host,
            raw.port,
            raw.url,
            raw.peers,
        )
    }
}

/// Read only a cached committed plan, never trigger a helper or allocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct GetWorkspaceServiceEnv {
    /// A service on this node, not a path or legacy project/session placement.
    pub target: WorkspaceScriptRef,
    /// A stale fence cannot cause a refresh with effects on Read.
    pub snapshot: ServiceSnapshotId,
}

/// Missing plan sources stay distinguishable from empty successful environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceEnvPlan {
    /// Read has no preparation effect and returns this until a real start plans.
    NotPrepared {
        /// Echo the exact request target, never create one from cwd.
        target: WorkspaceScriptRef,
        /// The observed metadata fence.
        snapshot: ServiceSnapshotId,
    },
    /// Only generated committed fields, with validated self and peer consistency.
    Ready {
        /// A projection cannot be passed as launch/admission authority.
        plan: ReadyServiceEnv,
    },
    /// A missing producer cannot be hidden by a successful empty map.
    Unavailable {
        /// Request identity is not a registry lease.
        target: WorkspaceScriptRef,
        /// No private error text or inherited credentials.
        reason: ServiceUnavailable,
    },
    /// No automatic operation follows a future plan state.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests;
