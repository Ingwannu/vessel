//! Typed metadata does not assert socket ownership, reachability or permission.

use serde::{Deserialize, Serialize};

use crate::workspace::ServicePort;

use super::ServiceProxyUrl;

/// Missing producers remain distinguishable from successful empty observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ServiceUnavailable {
    /// No qualified canonical registry reader is available.
    RegistryUnavailable,
    /// Runtime observation or admission has not been wired.
    RuntimeUnavailable,
    /// The registry blocks automatic project-file execution.
    AutomationBlocked,
    /// The exact current commands are not approved.
    ApprovalRequired,
    /// No usable project configuration was decoded.
    InvalidConfiguration,
    /// A committed allocation plan is unavailable.
    PlanUnavailable,
    /// Complete actual-run/socket ownership evidence is missing.
    OwnershipUnavailable,
    /// A previously complete observation is no longer current.
    ObservationExpired,
    /// TCP health cannot establish an HTTP(S) scheme.
    SchemeUnavailable,
    /// The dedicated proxy listener is unavailable.
    ListenerUnavailable,
    /// No qualified physical workspace incarnation exists.
    IncarnationUnavailable,
    /// Private settings cannot be read or persisted safely.
    SettingsUnavailable,
    /// The canonical workspace cannot admit this work now.
    WorkspaceInactive,
    /// A newer observation must not enable an old client's controls.
    #[serde(other)]
    Unknown,
}

/// Which rule selected a planned port, never a listener attestation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ServicePortSource {
    /// The approved definition explicitly selected this port.
    Explicit,
    /// The approved direct program selected it.
    Helper,
    /// The approved inclusive range selected it.
    Range,
    /// A kernel candidate was reserved by the planner.
    Ephemeral,
    /// Unknown provenance is not execution authority.
    #[serde(other)]
    Unknown,
}

/// A declared/reserved port and an observed listener are different facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServicePortPlan {
    /// Read does not cause allocation to happen.
    Unplanned,
    /// A committed planner reservation, not a listening socket.
    Reserved {
        /// Reuse the validated TCP port owner.
        port: ServicePort,
        /// The rule that selected this reservation.
        source: ServicePortSource,
    },
    /// Missing inputs remain explicit.
    Unavailable {
        /// A typed cause instead of a private error string.
        reason: ServiceUnavailable,
    },
    /// New observations must not enable operations implicitly.
    #[serde(other)]
    Unknown,
}

/// TCP probe observations cannot grant a scheme or private browser lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceHealth {
    /// No eligible route observation has started a health clock.
    NotObserved,
    /// The observation grace/probe result is still pending.
    Pending,
    /// The current TCP health policy observed success.
    Healthy,
    /// The policy's consecutive failure threshold was reached.
    Unhealthy,
    /// A missing producer is distinct from a failed TCP connection.
    Unavailable {
        /// Safe diagnostic classification only.
        reason: ServiceUnavailable,
    },
    /// Do not coerce a future health state into healthy.
    #[serde(other)]
    Unknown,
}

/// A node's loopback URL is not the remote app's own machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ServiceLinkAccess {
    /// Usable only in the node's network context.
    NodeLoopback,
    /// Explicitly consented publication without Vessel authentication.
    AnonymousPublic,
    /// A newer access class cannot be opened automatically.
    #[serde(other)]
    Unknown,
}

/// Link metadata alone cannot register a route or authorize Preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ServiceLink {
    /// Native consumers must distinguish node-only from public metadata.
    pub access: ServiceLinkAccess,
    /// The validated origin contains no node credentials.
    pub url: ServiceProxyUrl,
}

/// Preserve both URLs; public-first preference is a projection, not stored state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "RawServiceProxyUrls")]
pub struct ServiceProxyUrls {
    #[serde(skip_serializing_if = "Option::is_none")]
    local: Option<ServiceLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    public: Option<ServiceLink>,
}

/// A partial/contradictory link projection is refused instead of repaired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidServiceProxyUrls {
    /// Use Unavailable rather than claim a planned route with no URL.
    #[error("a service URL projection contains at least one link")]
    Empty,
    /// An app must not open node-loopback metadata as a public URL.
    #[error("service URL access or loopback address does not match its slot")]
    Access,
}

impl ServiceProxyUrls {
    /// Keep the role of each optional link inseparable from its slot.
    ///
    /// # Errors
    /// Empty projections and contradictory/unknown access classes refuse.
    pub fn new(
        local: Option<ServiceLink>,
        public: Option<ServiceLink>,
    ) -> Result<Self, InvalidServiceProxyUrls> {
        if local.is_none() && public.is_none() {
            return Err(InvalidServiceProxyUrls::Empty);
        }
        if local.as_ref().is_some_and(|link| {
            link.access != ServiceLinkAccess::NodeLoopback || !link.url.is_node_loopback()
        }) || public.as_ref().is_some_and(|link| {
            link.access != ServiceLinkAccess::AnonymousPublic || link.url.is_node_loopback()
        }) {
            return Err(InvalidServiceProxyUrls::Access);
        }
        Ok(Self { local, public })
    }

    /// Retain node-only reach semantics for a remote consumer.
    #[must_use]
    pub const fn local(&self) -> Option<&ServiceLink> {
        self.local.as_ref()
    }

    /// Publication metadata remains distinct from an effective route.
    #[must_use]
    pub const fn public(&self) -> Option<&ServiceLink> {
        self.public.as_ref()
    }

    /// The reference prefers public metadata when available.
    #[must_use]
    pub fn preferred(&self) -> Option<&ServiceLink> {
        self.public.as_ref().or(self.local.as_ref())
    }
}

#[derive(Deserialize)]
struct RawServiceProxyUrls {
    #[serde(default)]
    local: Option<ServiceLink>,
    #[serde(default)]
    public: Option<ServiceLink>,
}

impl TryFrom<RawServiceProxyUrls> for ServiceProxyUrls {
    type Error = InvalidServiceProxyUrls;

    fn try_from(raw: RawServiceProxyUrls) -> Result<Self, Self::Error> {
        Self::new(raw.local, raw.public)
    }
}

/// Planned metadata cannot be mistaken for actual registration/reachability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceProxyState {
    /// No listener/consent/qualified route is available.
    Unavailable {
        /// A safe missing-producer classification.
        reason: ServiceUnavailable,
    },
    /// May be used in generated environment before a process listens.
    Planned {
        /// The node constructs both roles from trusted configuration.
        urls: ServiceProxyUrls,
    },
    /// The node's complete route observation, still not a Preview lease.
    Registered {
        /// A fresh complete producer, not the caller, supplies this metadata.
        urls: ServiceProxyUrls,
    },
    /// No automatic action for an unrecognized observation.
    #[serde(other)]
    Unknown,
}

/// Independent status facts are metadata; runtime authority stays node-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ServiceObservation {
    /// Declaration/planner state, never a socket observation.
    pub port_plan: ServicePortPlan,
    /// Pending/unavailable must not be displayed as successful health.
    pub health: ServiceHealth,
    /// URL projection and registration remain distinct.
    pub proxy: ServiceProxyState,
}

#[cfg(test)]
mod tests;
