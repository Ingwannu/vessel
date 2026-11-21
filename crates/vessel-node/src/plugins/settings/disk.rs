//! A settings document on disk (ADR 0115, "Settings"): Paseo's envelope,
//! its revision, and the node's reads and owner-only writes through
//! `vessel-store`. Split from `settings.rs` to keep each file readable.

use std::fmt::Write as _;

use sha2::{Digest as _, Sha256};
use vessel_proto::RawJson;
use vessel_proto::plugins::{SettingsRevision, SettingsVersion};

use super::super::rows::blocking;
use super::super::{PluginError, Plugins};
use super::Target;

/// The stored envelope (Paseo's `{version, values}`).
#[derive(serde::Deserialize)]
struct Envelope {
    version: SettingsVersion,
    values: RawJson,
}

/// The envelope as written.
#[derive(serde::Serialize)]
struct EnvelopeOut<'a> {
    version: SettingsVersion,
    values: &'a RawJson,
}

/// A document as it is on disk.
pub(super) enum Stored {
    Missing,
    Unreadable(SettingsRevision),
    Saved {
        revision: SettingsRevision,
        version: SettingsVersion,
        values: RawJson,
    },
}

impl Stored {
    pub(super) fn revision(&self) -> SettingsRevision {
        match self {
            Self::Missing => SettingsRevision::missing(),
            Self::Unreadable(revision) | Self::Saved { revision, .. } => revision.clone(),
        }
    }
}

impl Plugins {
    /// The document on disk.
    pub(super) async fn settings_stored(&self, target: &Target) -> Result<Stored, PluginError> {
        let data = self.inner.data.clone();
        let (plugin, settings) = (target.plugin.clone(), target.settings.clone());
        let bytes = blocking(move || data.read_plugin_settings(&plugin, &settings))
            .await?
            .map_err(|error| PluginError::SettingsStore(error.to_string()))?;
        let Some(bytes) = bytes else {
            return Ok(Stored::Missing);
        };
        let revision = revision_of(&bytes);
        Ok(match serde_json::from_slice::<Envelope>(&bytes) {
            Ok(Envelope { version, values }) => Stored::Saved {
                revision,
                version,
                values,
            },
            Err(_unreadable) => Stored::Unreadable(revision),
        })
    }

    /// Write `values` at the declared version; its new revision.
    pub(super) async fn settings_persist(
        &self,
        target: &Target,
        values: &RawJson,
    ) -> Result<SettingsRevision, PluginError> {
        let raw = serde_json::to_vec(&EnvelopeOut {
            version: target.version,
            values,
        })
        .map_err(|error| PluginError::SettingsStore(error.to_string()))?;
        let revision = revision_of(&raw);
        let data = self.inner.data.clone();
        let (plugin, settings) = (target.plugin.clone(), target.settings.clone());
        blocking(move || data.write_plugin_settings(&plugin, &settings, &raw))
            .await?
            .map_err(|error| PluginError::SettingsStore(error.to_string()))?;
        Ok(revision)
    }
}

/// The sha256 of `bytes` in lowercase hex.
fn revision_of(bytes: &[u8]) -> SettingsRevision {
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _infallible = write!(hex, "{byte:02x}");
    }
    SettingsRevision::from_sha256_hex(&hex).unwrap_or_else(SettingsRevision::missing)
}
