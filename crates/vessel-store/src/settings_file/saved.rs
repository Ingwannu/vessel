//! The private disk model, separate from a caller's generic wire patch.
//! No service/plugin schema or mutation authority is introduced here.

use std::fmt;

use serde::{Deserialize, Serialize};
use vessel_proto::parking::ParkAfter;
use vessel_proto::restart::SetRestartRecovery;
use vessel_proto::settings::{SetAgentDrivers, SetNodeSettings};

/// A saved document cannot accidentally become a generic Read/wire reply.
/// Its disk codec remains inside Store; only the generic projection escapes.
///
/// ```compile_fail
/// let saved = vessel_store::SavedNodeSettings::default();
/// // Private disk envelopes are deliberately not wire-serializable.
/// let _ = serde_json::to_string(&saved);
/// ```
#[derive(Clone, Default, PartialEq, Eq)]
pub struct SavedNodeSettings {
    document: Document,
}

// Explicit disk ownership preserves field order and the existing canonical
// bytes while keeping future protected namespaces out of a generic wire patch.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Document {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    park_after: Option<ParkAfter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    drivers: Option<SetAgentDrivers>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    restart_recovery: Option<SetRestartRecovery>,
}

impl fmt::Debug for SavedNodeSettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SavedNodeSettings { .. }")
    }
}

impl SavedNodeSettings {
    /// Obtain only the current generic saved fields, never a private disk record.
    #[must_use]
    pub fn public_settings(&self) -> SetNodeSettings {
        SetNodeSettings {
            park_after: self.document.park_after,
            drivers: self.document.drivers,
            restart_recovery: self.document.restart_recovery.clone(),
        }
    }

    /// Replace the complete generic snapshot in an owned candidate document.
    /// This is not a live mutation: Node's one owner validates, persists and
    /// publishes the candidate. Future private fields must remain intact here.
    #[must_use]
    pub fn with_public_settings(mut self, public: SetNodeSettings) -> Self {
        let SetNodeSettings {
            park_after,
            drivers,
            restart_recovery,
        } = public;
        self.document.park_after = park_after;
        self.document.drivers = drivers;
        self.document.restart_recovery = restart_recovery;
        self
    }

    /// Keep tolerant file decoding at the disk boundary.
    ///
    /// # Errors
    /// Keep the concrete parse cause private behind the canonical diagnostic edge.
    pub(super) fn decode(bytes: &[u8]) -> Result<Self, super::SettingsDecodeFailure> {
        super::decode::settings(bytes).map(|document| Self { document })
    }

    /// Preserve the existing canonical writer without exposing serde on Self.
    ///
    /// # Errors
    /// Retain any encoding failure for the existing Store error mapper.
    pub(super) fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec_pretty(&self.document)
    }

    /// Keep future-field comparison trees inside the file decoding edge.
    ///
    /// # Errors
    /// Preserve the canonical representation's encoding cause.
    pub(super) fn comparable(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(&self.document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_private_codec_preserves_current_canonical_wire_patch_bytes() {
        for input in [
            b"{}".as_slice(),
            br#"{"park_after":null,"drivers":null}"#,
            br#"{"park_after":{"kind":"never"}}"#,
            br#"{"park_after":{"kind":"future"}}"#,
            br#"{"drivers":{}}"#,
            br#"{"park_after":{"kind":"idle","minutes":15},"drivers":{"codex":"direct"}}"#,
            br#"{"drivers":{"claude_code":"acp","codex":"acp","open_code":"acp"}}"#,
            br#"{"drivers":{"codex":"future"},"future":{"nested":true}}"#,
            br#"{"restart_recovery":{"enabled":true,"prompt":"  Keep exact.\n"}}"#,
            br#"{"park_after":{"kind":"never"},"restart_recovery":{"enabled":false,"prompt":"Keep while off."}}"#,
            br#"{"restart_recovery":{"enabled":null,"prompt":null,"future":true}}"#,
        ] {
            let public: SetNodeSettings = serde_json::from_slice(input).unwrap();
            let saved = SavedNodeSettings::decode(input).unwrap();
            assert_eq!(saved.public_settings(), public);
            assert_eq!(
                saved.encode().unwrap(),
                serde_json::to_vec_pretty(&public).unwrap()
            );
            assert_eq!(
                saved.comparable().unwrap(),
                serde_json::to_value(public).unwrap()
            );
            assert_eq!(
                SavedNodeSettings::decode(&saved.encode().unwrap()).unwrap(),
                saved
            );
        }
    }
}
