//! A single physical read binds typed settings and future-field preservation.
//! JSON trees stay at this file edge; callers receive only the opaque model.

use std::io;

use super::{SavedNodeSettings, SettingsFileError, covered, decode};
use crate::{DataDir, StoreError};

/// Whether the first changed write must preserve the original file first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsDisposition {
    /// Every decoded value is represented by this build's disk model.
    Current,
    /// A future or uncertain value must not be lost on the first changed write.
    PreserveBeforeWrite,
}

/// Typed content and its preservation decision from the same physical bytes.
#[derive(Debug)]
pub struct SettingsFileSnapshot {
    saved: Option<SavedNodeSettings>,
    disposition: SettingsDisposition,
}

impl SettingsFileSnapshot {
    /// Transfer the exact decoded document and its preservation obligation.
    #[must_use]
    pub fn into_parts(self) -> (Option<SavedNodeSettings>, SettingsDisposition) {
        (self.saved, self.disposition)
    }
}

impl DataDir {
    /// Read once for startup policy and the later settings mutation owner.
    /// Unlike a separate coverage query, this cannot compare a replacement's
    /// bytes with the document already admitted for this startup.
    ///
    /// # Errors
    /// Preserve actual I/O and the canonical private-cause decode diagnostics.
    pub fn read_settings_snapshot(&self) -> Result<SettingsFileSnapshot, SettingsFileError> {
        let path = self.settings_path();
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(SettingsFileSnapshot {
                    saved: None,
                    disposition: SettingsDisposition::Current,
                });
            }
            Err(error) => return Err(StoreError::io(&path)(error).into()),
        };
        let saved = SavedNodeSettings::decode(&bytes)
            .map_err(|source| SettingsFileError::Parse { path, source })?;
        // Unknown deeply nested values can defeat the comparison tree while
        // tolerant typed decoding still succeeds. Preserve the old conservative
        // rule: uncertainty requires a copy, never destructive confidence.
        let disposition = match coverage(&bytes, &saved) {
            Ok(disposition) => disposition,
            Err(error) => {
                tracing::debug!(%error, "settings coverage is uncertain; preserve before writing");
                SettingsDisposition::PreserveBeforeWrite
            }
        };
        Ok(SettingsFileSnapshot {
            saved: Some(saved),
            disposition,
        })
    }
}

pub(super) fn coverage(
    bytes: &[u8],
    saved: &SavedNodeSettings,
) -> Result<SettingsDisposition, super::SettingsDecodeFailure> {
    let file: serde_json::Value = decode::settings(bytes)?;
    // The typed model's encoder has no foreign fallible value implementation.
    // Keep any concrete failure sealed in the same diagnostic wrapper anyway.
    let known = saved.comparable().map_err(decode::encoding)?;
    Ok(if covered(&file, &known) {
        SettingsDisposition::Current
    } else {
        SettingsDisposition::PreserveBeforeWrite
    })
}

#[cfg(test)]
mod tests;
