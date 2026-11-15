//! Typed explicit mapping from a compatibility provider key to actual node facts.

use super::super::LaunchProfileClientError;
use crate::NodeClient;
use std::{collections::HashSet, sync::Arc};
use vessel_proto::NodeId;
use vessel_proto::launch_profiles::{
    LaunchAccountKind, LaunchCatalogue, LaunchProfileName, LegacyFavorites, LegacyProviderKey,
};

/// Loading/busy are the only source retry states; ordinary errors are not retried.
#[derive(Debug)]
pub enum LegacyCatalogueState {
    /// The caller's actual catalogue source is still loading.
    Loading,
    /// The node explicitly refused finite probe admission as busy.
    Busy,
    /// A terminal actual node catalogue for this provider mapping.
    Ready(MigrationCatalogue),
}

/// A reported catalogue plus validated public provider identity/label.
#[derive(Debug, Clone)]
pub struct MigrationCatalogue {
    node: NodeId,
    provider: LegacyProviderKey,
    label: LaunchProfileName,
    catalogue: LaunchCatalogue,
}
impl MigrationCatalogue {
    /// Keep an explicit source mapping; this helper invents no provider or
    /// native/ACP id conversion and accepts no draft-dependent catalogue.
    ///
    /// # Errors
    /// Account identity is unknown or the probe applied nondefault choices.
    pub fn new(
        node: NodeId,
        provider: LegacyProviderKey,
        label: LaunchProfileName,
        catalogue: LaunchCatalogue,
    ) -> Result<Self, LaunchProfileClientError> {
        if !catalogue.selection().options().as_slice().is_empty()
            || catalogue.account().kind() == LaunchAccountKind::Unknown
        {
            return Err(LaunchProfileClientError::LegacyCatalogueUnusable);
        }
        Ok(Self {
            node,
            provider,
            label,
            catalogue,
        })
    }
    /// Authenticated responding node identity used for this catalogue.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.node
    }
    /// Exact local compatibility provider key; not an execution authority.
    #[must_use]
    pub const fn provider(&self) -> &LegacyProviderKey {
        &self.provider
    }
    /// Actual source label, already bounded for a profile name.
    #[must_use]
    pub const fn label(&self) -> &LaunchProfileName {
        &self.label
    }
    /// The node's actual controls, canonical account and exact selection.
    #[must_use]
    pub const fn catalogue(&self) -> &LaunchCatalogue {
        &self.catalogue
    }
}

/// One finite unique-provider catalogue set, independent of global caches.
#[derive(Debug, Clone)]
pub struct MigrationCatalogues(Arc<[MigrationCatalogue]>);
impl MigrationCatalogues {
    /// At most one catalogue for each bounded compatibility input entry.
    pub const MAX_CATALOGUES: usize = LegacyFavorites::MAX_ENTRIES;
    /// Actual mappings in supplied order.
    #[must_use]
    pub fn as_slice(&self) -> &[MigrationCatalogue] {
        &self.0
    }
}
impl TryFrom<Vec<MigrationCatalogue>> for MigrationCatalogues {
    type Error = LaunchProfileClientError;
    fn try_from(values: Vec<MigrationCatalogue>) -> Result<Self, Self::Error> {
        if values.len() > Self::MAX_CATALOGUES {
            return Err(LaunchProfileClientError::LegacyCatalogueCapacity);
        }
        let mut seen = HashSet::new();
        if values.iter().any(|value| !seen.insert(value.provider())) {
            return Err(LaunchProfileClientError::LegacyCatalogueDuplicate);
        }
        Ok(Self(Arc::from(values)))
    }
}

impl NodeClient {
    /// Resolve one explicit provider mapping and tag it with this connection's
    /// actual node identity, so a stale reply from another machine is rejected.
    ///
    /// # Errors
    /// Normal editor probe refusals, unknown account identity, or a catalogue
    /// that applied draft choices instead of reporting its unmodified controls.
    pub async fn migration_catalogue(
        &self,
        provider: LegacyProviderKey,
        label: LaunchProfileName,
        selection: vessel_proto::launch_profiles::LaunchSelection,
    ) -> Result<MigrationCatalogue, LaunchProfileClientError> {
        let catalogue = self.launch_profile_options(selection).await?;
        MigrationCatalogue::new(self.node().id, provider, label, catalogue)
    }
}
