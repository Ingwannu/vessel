//! Dedicated Admin DTOs; no private disk schema, writer or grant authority.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::workspace::CommandsDigest;

use super::{
    ServiceGateway, ServiceProxyUrl, ServiceSnapshotId, ServiceUnavailable,
    ServicesSettingsRevision, WorkspaceScriptRef,
};

/// Bound the node's explicit publication choices independently of page size.
pub const MAX_PUBLIC_SERVICE_CHOICES: usize = 256;

/// A requested choice is distinct from what ingress owners have settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ServiceExposureChoice {
    /// No persisted anonymous-public consent.
    Private,
    /// Consent alone is not fresh run/socket/incarnation proof.
    AnonymousPublic,
}

/// Actual publication projection, never an authorization token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceExposureEffective {
    /// No anonymous route is effective.
    Private,
    /// A requested choice cannot take effect for a typed reason.
    Inactive {
        /// Missing proof must not silently fall back to publication.
        reason: ServiceUnavailable,
    },
    /// The trusted node projection observed an effective public route.
    Registered {
        /// An origin is still metadata, never a private Preview lease.
        url: ServiceProxyUrl,
    },
    /// Actual HTTP/WS owners have not finished draining.
    Revoking,
    /// A future state cannot be coerced into effective publication.
    #[serde(other)]
    Unknown,
}

/// A private Admin projection without the stored canonical incarnation value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ServiceExposureView {
    /// The exact workspace/name pair is not inferred from a directory.
    pub target: WorkspaceScriptRef,
    /// What the dedicated Admin setter last requested.
    pub choice: ServiceExposureChoice,
    /// Requested and effective state may differ while revocation settles.
    pub effective: ServiceExposureEffective,
}

/// Bounded, unique Admin view; ordinary node settings Read never contains it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "RawServicesAdminSettings")]
pub struct ServicesAdminSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<ServicesSettingsRevision>,
    gateway: ServiceGateway,
    exposures: Vec<ServiceExposureView>,
}

/// Complete borrowed projection forces display consumers to name every field.
#[derive(Debug, Clone, Copy)]
pub struct ServicesAdminSettingsView<'a> {
    /// Absence is checked absence at mutation, never unconditional overwrite.
    pub revision: Option<ServicesSettingsRevision>,
    /// Changing gateway invalidates existing per-service consent.
    pub gateway: &'a ServiceGateway,
    /// No private serialized binding or environment credentials are exposed.
    pub exposures: &'a [ServiceExposureView],
}

/// Refuse oversized or ambiguous per-service choices without dropping entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidServicesAdminSettings {
    /// One named count bound covers constructors, decoding and diagnostics.
    #[error("service settings contain at most {max} choices", max = MAX_PUBLIC_SERVICE_CHOICES)]
    TooMany,
    /// One exact target cannot have contradictory requested choices.
    #[error("service settings repeat a workspace script")]
    Duplicate,
}

impl ServicesAdminSettings {
    /// Preserve all entries or refuse the whole projection at its decode edge.
    ///
    /// # Errors
    /// More than the publication count bound, or duplicate exact targets.
    pub fn new(
        revision: Option<ServicesSettingsRevision>,
        gateway: ServiceGateway,
        exposures: Vec<ServiceExposureView>,
    ) -> Result<Self, InvalidServicesAdminSettings> {
        if exposures.len() > MAX_PUBLIC_SERVICE_CHOICES {
            return Err(InvalidServicesAdminSettings::TooMany);
        }
        let mut seen = HashSet::with_capacity(exposures.len());
        for exposure in &exposures {
            if !seen.insert(&exposure.target) {
                return Err(InvalidServicesAdminSettings::Duplicate);
            }
        }
        Ok(Self {
            revision,
            gateway,
            exposures,
        })
    }

    /// Expose every descriptive field without allowing independent mutation.
    #[must_use]
    pub fn view(&self) -> ServicesAdminSettingsView<'_> {
        let Self {
            revision,
            gateway,
            exposures,
        } = self;
        ServicesAdminSettingsView {
            revision: *revision,
            gateway,
            exposures,
        }
    }
}

#[derive(Deserialize)]
struct RawServicesAdminSettings {
    #[serde(default)]
    revision: Option<ServicesSettingsRevision>,
    gateway: ServiceGateway,
    exposures: Vec<ServiceExposureView>,
}

impl TryFrom<RawServicesAdminSettings> for ServicesAdminSettings {
    type Error = InvalidServicesAdminSettings;

    fn try_from(raw: RawServicesAdminSettings) -> Result<Self, Self::Error> {
        Self::new(raw.revision, raw.gateway, raw.exposures)
    }
}

/// No ordinary Read method may return this dedicated Admin projection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetServiceSettings {}

/// Explicit settings CAS, not a node bind or default public-service grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SetServiceGateway {
    /// None means the services slot must still be absent.
    pub expected: Option<ServicesSettingsRevision>,
    /// A changed value clears prior consent and drains the old ingress.
    pub gateway: ServiceGateway,
}

/// The caller must consciously acknowledge the anonymous publication warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PublicServiceAcknowledgement {
    /// The service becomes reachable without Vessel authentication.
    ReachableWithoutVesselAuthentication,
}

/// Unknown mutation choices refuse rather than implicitly choosing a default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServiceExposureChange {
    /// Remove the private consent record; drain old HTTP/WS owners.
    Private,
    /// The node derives the mandatory canonical binding itself before any write.
    AnonymousPublic {
        /// A metadata fence, never an actual-run token.
        snapshot: ServiceSnapshotId,
        /// The current approved definition must still match.
        definition: CommandsDigest,
        /// No bool/default/absence can acknowledge anonymous exposure.
        acknowledgement: PublicServiceAcknowledgement,
    },
}

/// Only the dedicated direct Admin setter may reach the private services slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SetServiceExposure {
    /// Exact local registry identity is rechecked at effect admission.
    pub target: WorkspaceScriptRef,
    /// None is a checked absence fence, not an overwrite option.
    pub expected: Option<ServicesSettingsRevision>,
    /// Public enable stays Unavailable before write without canonical incarnation.
    pub change: ServiceExposureChange,
}

#[cfg(test)]
mod tests;
