//! Deterministic migration metadata identity, never a bearer credential.

use super::{LaunchProfileId, Uuid};
use crate::launch_profiles::{InvalidLaunchValue, LegacyFavorite};

/// A fixed namespace dedicated to the version-one legacy-favorites bridge.
/// Its identity and canonical tuple encoding must never change after release.
const LEGACY_FAVORITES_NAMESPACE: Uuid = Uuid::from_u128(0xa2d6_b727_54e9_49d4_8f85_5c1f_7657_8ce2);

impl LaunchProfileId {
    /// Stable metadata id over compact UTF-8 JSON `[provider,model]`, using
    /// a dedicated `UUIDv5` namespace (ADR 0076). This is not a credential
    /// hash and is never used for devices, sessions or authentication.
    ///
    /// # Errors
    /// The canonical typed pair could not be serialized; source is retained.
    pub fn for_legacy_favorite(pair: &LegacyFavorite) -> Result<Self, InvalidLaunchValue> {
        let encoded = serde_json::to_vec(&(pair.provider(), pair.model()))
            .map_err(InvalidLaunchValue::Encoding)?;
        Ok(Self(Uuid::new_v5(&LEGACY_FAVORITES_NAMESPACE, &encoded)))
    }
}
