//! The full local saved record, independently of registered RPC patch fields.
//!
//! [Decision Log]
//! - Purpose: staging recovery must preserve its saved policy when an older
//!   ordinary request changes parking/drivers; it must not register an input.
//! - Choice: a store-owned record, with the legacy `DataDir` APIs retained as
//!   compatibility views. Both views share `settings_file`'s codec, coverage
//!   and atomic/private replacement. Node settings still owns precedence.
//! - Cost: explicit conversion from the wire patch. Its exhaustive pattern
//!   makes future wire fields a compile-time decision rather than losing them.

use serde::{Deserialize, Serialize};
use vessel_proto::parking::ParkAfter;
use vessel_proto::restart::SetRestartRecovery;
use vessel_proto::settings::{SetAgentDrivers, SetNodeSettings};

use crate::{DataDir, SettingsFileError, StoreError};

/// Settings chosen locally, including policy not yet registered on the wire.
/// Every field is optional so an unchosen setting keeps following its default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedNodeSettings {
    /// The stored parking choice, before flags/fallback are applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub park_after: Option<ParkAfter>,
    /// The stored driver choices; later/unknown values keep the legacy policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drivers: Option<SetAgentDrivers>,
    /// A local recovery patch, shared with the explicitly versioned wire input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_recovery: Option<SetRestartRecovery>,
}

impl From<SetNodeSettings> for SavedNodeSettings {
    fn from(change: SetNodeSettings) -> Self {
        let SetNodeSettings {
            park_after,
            drivers,
            restart_recovery,
        } = change;
        Self {
            park_after,
            drivers,
            restart_recovery,
        }
    }
}

impl DataDir {
    /// Read the complete saved record without admitting a new RPC field.
    /// Decode once at the same edge as the compatibility view; malformed
    /// record diagnostics redact values while retaining the typed source.
    ///
    /// # Errors
    /// The file cannot be read or its typed values do not parse.
    pub fn read_settings_record(&self) -> Result<Option<SavedNodeSettings>, SettingsFileError> {
        self.read_settings_as()
            .map_err(SettingsFileError::for_record)
    }

    /// Whether the file has fields/values the full saved record cannot retain.
    /// The node copies such a file aside before its first replacing write.
    ///
    /// # Errors
    /// The file cannot be read or it is not JSON.
    pub fn settings_record_holds_more_than(
        &self,
        known: &SavedNodeSettings,
    ) -> Result<bool, StoreError> {
        self.settings_contain_more_than(known)
    }

    /// Replace the complete saved record, owner-only and atomically. The
    /// node's one settings engine serializes this with apply/intent cleanup.
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn write_settings_record(&self, settings: &SavedNodeSettings) -> Result<(), StoreError> {
        self.write_settings_as(settings)
    }
}

#[cfg(test)]
mod tests;
