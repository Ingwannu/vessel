//! One owning node's immutable public declaration snapshot, without management
//! paths, private logs, registry credentials or executable preparation commands.
use super::{
    ClientDocument, PluginAddress, PluginJsonError,
    budget::{EncodingError, encoded_bytes},
};
use crate::{NodeId, ProtocolVersion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// All installations on a node share this catalog admission budget.
pub const MAX_CATALOG_INSTALLATIONS: usize = 128;
/// Aggregate themes per node; no independent global theme-choice limit.
pub const MAX_CATALOG_THEMES: usize = 256;
/// Complete canonical catalog, in addition to each document's individual cap.
pub const MAX_CATALOG_BYTES: usize = 4 * 1024 * 1024;

/// Published only after an installation's complete immutable document validates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClientInstallation {
    address: PluginAddress,
    required_client: ProtocolVersion,
    document: ClientDocument,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InstallationInput {
    address: PluginAddress,
    required_client: ProtocolVersion,
    document: ClientDocument,
}

impl ClientInstallation {
    /// Pair declarations with exact owning host/run and a nonzero client floor.
    /// # Errors
    /// Refuses an invalid protocol requirement; declarations are already checked.
    pub fn new(
        address: PluginAddress,
        required_client: ProtocolVersion,
        document: ClientDocument,
    ) -> Result<Self, CatalogError> {
        if required_client.get() == 0 {
            return Err(CatalogError::Requirement);
        }
        Ok(Self {
            address,
            required_client,
            document,
        })
    }
    /// Exact owner identity, never a display-label deduplication key.
    #[must_use]
    pub const fn address(&self) -> &PluginAddress {
        &self.address
    }
    /// The author-declared minimum client protocol; checked before publication.
    #[must_use]
    pub const fn required_client(&self) -> ProtocolVersion {
        self.required_client
    }
    /// Validated native data, with no executable client callbacks.
    #[must_use]
    pub const fn document(&self) -> &ClientDocument {
        &self.document
    }
}
impl<'de> Deserialize<'de> for ClientInstallation {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input = InstallationInput::deserialize(deserializer).map_err(|_untrusted| {
            serde::de::Error::custom("invalid plugin installation structure")
        })?;
        Self::new(input.address, input.required_client, input.document)
            .map_err(serde::de::Error::custom)
    }
}

/// A complete node snapshot; missing installations are authoritative withdrawal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClientCatalog {
    node: NodeId,
    installations: Vec<ClientInstallation>,
    #[serde(skip)]
    encoded_bytes: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogInput {
    node: NodeId,
    installations: Vec<ClientInstallation>,
}

/// Catalog rejection is atomic and safe for generic serde or direct-byte callers.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// Named entry checks the byte cap before parsing.
    #[error("plugin catalog exceeds {MAX_CATALOG_BYTES} bytes")]
    Bytes,
    /// Concrete parser diagnostic, sealed before it can quote arbitrary input.
    #[error("invalid plugin catalog JSON")]
    Decode(#[source] PluginJsonError),
    /// Concrete encoding cause; the counting sink does not retain source text.
    #[error("plugin catalog could not be encoded")]
    Encode(#[source] PluginJsonError),
    /// A zero floor cannot designate any compatible client.
    #[error("plugin installation requires a nonzero client protocol")]
    Requirement,
    /// One host must not introduce another host's installation.
    #[error("plugin catalog contains an installation owned by another node")]
    Owner,
    /// No order-dependent choice between installations sharing one logical id.
    #[error("plugin catalog contains a duplicate installation")]
    Duplicate,
    /// Catalog-wide count admission.
    #[error("plugin catalog exceeds {MAX_CATALOG_INSTALLATIONS} installations")]
    Installations,
    /// Themes share a whole-node budget, not a per-plugin multiplication.
    #[error("plugin catalog exceeds {MAX_CATALOG_THEMES} themes")]
    Themes,
}

impl ClientCatalog {
    /// Decode bounded bytes; no raw parser text escapes the safe cause boundary.
    /// # Errors
    /// Refuses excessive, malformed, duplicate, incompatible-owner snapshots.
    pub fn decode(bytes: &[u8]) -> Result<Self, CatalogError> {
        if bytes.len() > MAX_CATALOG_BYTES {
            return Err(CatalogError::Bytes);
        }
        let input: CatalogInput = serde_json::from_slice(bytes)
            .map_err(|cause| CatalogError::Decode(PluginJsonError::new(cause)))?;
        Self::new(input.node, input.installations)
    }
    /// Build the same fully checked snapshot without going through JSON bytes.
    /// # Errors
    /// Count, owner, duplicate and canonical encoded-size admission apply equally.
    pub fn new(node: NodeId, installations: Vec<ClientInstallation>) -> Result<Self, CatalogError> {
        if installations.len() > MAX_CATALOG_INSTALLATIONS {
            return Err(CatalogError::Installations);
        }
        let mut ids = BTreeSet::new();
        let mut themes = 0_usize;
        for installation in &installations {
            if installation.address.node != node {
                return Err(CatalogError::Owner);
            }
            if !ids.insert(&installation.address.plugin) {
                return Err(CatalogError::Duplicate);
            }
            themes = themes.saturating_add(installation.document.themes().len());
            if themes > MAX_CATALOG_THEMES {
                return Err(CatalogError::Themes);
            }
        }
        let mut catalog = Self {
            node,
            installations,
            encoded_bytes: 0,
        };
        catalog.encoded_bytes =
            encoded_bytes(&catalog, MAX_CATALOG_BYTES).map_err(|error| match error {
                EncodingError::Exceeded => CatalogError::Bytes,
                EncodingError::Encode(cause) => CatalogError::Encode(cause),
            })?;
        Ok(catalog)
    }
    /// Node identity must agree with the authenticated welcome, not an alias.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.node
    }
    /// Borrow exactly this node's validated installation order.
    #[must_use]
    pub fn installations(&self) -> &[ClientInstallation] {
        &self.installations
    }
    /// Cached compact serialized size for client-wide retained-data admission.
    #[must_use]
    pub const fn encoded_bytes(&self) -> usize {
        self.encoded_bytes
    }
}
impl<'de> Deserialize<'de> for ClientCatalog {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input = CatalogInput::deserialize(deserializer)
            .map_err(|_untrusted| serde::de::Error::custom("invalid plugin catalog structure"))?;
        Self::new(input.node, input.installations).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
