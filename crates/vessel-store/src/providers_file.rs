//! Provider configuration file boundary (ADR 0081). Commands and environment
//! values stay local. Preserve unreadable/later files before any atomic write.
//! No JSON tree escapes the existing settings-file comparison operation.

use crate::{DataDir, SetAside, StoreError, agents_file, files, settings_file};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read as _},
    path::PathBuf,
};
use vessel_proto::providers::ProviderOverrides;

mod failure;
pub use failure::ProvidersFileFailure;

const PROVIDERS_FILE: &str = "providers.json";
/// A local decode bound independent from the node's transport frame budget.
pub const MAX_PROVIDER_FILE_BYTES: u64 = 4 * 1024 * 1024;

/// Startup status never exposes the preserved file's private contents or path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderFileState {
    /// No override file; use manifest defaults.
    Missing,
    /// The complete typed generation was read.
    Present,
    /// Unreadable bytes were safely moved aside before defaults were selected.
    Recovered,
}

/// Exactly one complete startup generation and its preservation outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProviders {
    /// Effective saved overrides, empty only for absent/recovered files.
    pub overrides: ProviderOverrides,
    /// A safe status for the node's local startup log.
    pub state: ProviderFileState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Document {
    #[serde(default)]
    providers: ProviderOverrides,
}

/// Storage errors retain their cause, but formatted diagnostics never echo
/// parser input, local path text or configuration values.
#[derive(thiserror::Error)]
pub enum ProvidersFileError {
    /// A filesystem operation failed.
    #[error("provider configuration could not be read or written")]
    Io(#[source] ProvidersFileFailure),
    /// Malformed typed input. The source stays private to the decoder boundary.
    #[error("provider configuration could not be decoded")]
    Decode(#[source] ProvidersFileFailure),
    /// Permission or link safety failed before any file content was read.
    #[error(
        "provider configuration must be an owner-only regular file in a trusted data directory"
    )]
    Unsafe,
    /// Refuse before retaining an unbounded input allocation.
    #[error("provider configuration exceeds {MAX_PROVIDER_FILE_BYTES} bytes")]
    TooLarge,
    /// Encoding failed before any replacement was attempted.
    #[error("provider configuration could not be encoded")]
    Encode(#[source] ProvidersFileFailure),
}

impl From<StoreError> for ProvidersFileError {
    fn from(cause: StoreError) -> Self {
        Self::Io(cause.into())
    }
}

impl std::fmt::Debug for ProvidersFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl DataDir {
    /// Load one startup generation using the settings-file recovery rule.
    /// Unsafe files remain untouched; failed preservation refuses startup.
    ///
    /// # Errors
    /// Unsafe ownership/modes or inability to preserve unreadable bytes.
    pub fn load_provider_overrides(
        &self,
        at: DateTime<Utc>,
    ) -> Result<LoadedProviders, ProvidersFileError> {
        match self.read_provider_overrides() {
            Ok(overrides) => {
                let state = if overrides.is_some() {
                    ProviderFileState::Present
                } else {
                    ProviderFileState::Missing
                };
                Ok(LoadedProviders {
                    overrides: overrides.unwrap_or_default(),
                    state,
                })
            }
            Err(ProvidersFileError::Unsafe) => Err(ProvidersFileError::Unsafe),
            Err(
                ProvidersFileError::Io(_)
                | ProvidersFileError::Decode(_)
                | ProvidersFileError::TooLarge
                | ProvidersFileError::Encode(_),
            ) => {
                let _preserved = settings_file::set_aside(
                    self.root(),
                    &self.providers_path(),
                    at,
                    SetAside::Unreadable,
                )?;
                Ok(LoadedProviders {
                    overrides: ProviderOverrides::default(),
                    state: ProviderFileState::Recovered,
                })
            }
        }
    }

    /// Local provider config path; never part of a reader snapshot.
    #[must_use]
    pub fn providers_path(&self) -> PathBuf {
        self.root().join(PROVIDERS_FILE)
    }

    /// Decode only a trusted local file. Unknown fields are tolerated, and a
    /// later save preserves the complete original bytes before replacing them.
    ///
    /// # Errors
    /// I/O, unsafe ownership/modes/links, malformed input or oversized data.
    pub fn read_provider_overrides(&self) -> Result<Option<ProviderOverrides>, ProvidersFileError> {
        let path = self.providers_path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(StoreError::io(&path)(error).into()),
        };
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || agents_file::unsafe_to_run(self.root(), &path)
                .map_err(StoreError::io(&path))?
                .is_some()
            || files::readable_by_others(&path).map_err(StoreError::io(&path))?
        {
            return Err(ProvidersFileError::Unsafe);
        }
        let mut bytes = Vec::new();
        let _bytes_read = fs::File::open(&path)
            .map_err(StoreError::io(&path))?
            .take(MAX_PROVIDER_FILE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(StoreError::io(&path))?;
        if !u64::try_from(bytes.len()).is_ok_and(|length| length <= MAX_PROVIDER_FILE_BYTES) {
            return Err(ProvidersFileError::TooLarge);
        }
        let document: Document = serde_json::from_slice(&bytes)
            .map_err(|cause| ProvidersFileError::Decode(cause.into()))?;
        Ok(Some(document.providers))
    }

    /// Preserve the old file when necessary, then atomically save one complete
    /// provider generation. Only the node holding its data-directory lock may
    /// call this method. Call from a blocking context, after registry validation.
    ///
    /// # Errors
    /// If preservation fails the original remains the authoritative file and
    /// replacement is refused. Unsafe files are never moved or overwritten.
    pub fn save_provider_overrides(
        &self,
        providers: &ProviderOverrides,
        at: DateTime<Utc>,
    ) -> Result<(), ProvidersFileError> {
        let path = self.providers_path();
        let document = Document {
            providers: providers.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&document)
            .map_err(|cause| ProvidersFileError::Encode(cause.into()))?;
        if !u64::try_from(bytes.len()).is_ok_and(|length| length <= MAX_PROVIDER_FILE_BYTES) {
            return Err(ProvidersFileError::TooLarge);
        }
        match self.read_provider_overrides() {
            Ok(Some(known)) => {
                let known = Document { providers: known };
                if settings_file::holds_more_than(&path, &known)? {
                    let _preserved =
                        settings_file::set_aside(self.root(), &path, at, SetAside::Later)?;
                }
            }
            Ok(None) => {}
            Err(ProvidersFileError::Unsafe) => return Err(ProvidersFileError::Unsafe),
            Err(
                ProvidersFileError::Io(_)
                | ProvidersFileError::Decode(_)
                | ProvidersFileError::TooLarge
                | ProvidersFileError::Encode(_),
            ) => {
                let _preserved =
                    settings_file::set_aside(self.root(), &path, at, SetAside::Unreadable)?;
            }
        }
        files::replace(&path, &bytes).map_err(StoreError::io(&path))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
