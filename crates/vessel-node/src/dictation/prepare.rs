//! Prepare one stream from a current validated settings snapshot and fresh secret.
//!
//! The registry obtains that snapshot only after retiring the prior generation.
//! This owner reuses the shared read gate and one TLS cache; it caches no key,
//! settings snapshot, destination or HTTP pool across starts.

use super::{
    endpoint::{ProviderError, Specification},
    jobs::Source,
    quota::Lease,
    roots,
    secret_read::{Admission, ReadError, Reader},
};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use vessel_proto::dictation::{
    DictationDecodeError, DictationProvider, DictationSwitch, KeyPresence, LocalModelId,
    SetDictation,
};

/// Keep preparation causes internal until the service's one fixed wire mapping.
#[derive(thiserror::Error)]
pub(super) enum PrepareError {
    #[error("the saved dictation configuration is not supported")]
    Configuration(#[source] DictationDecodeError),
    #[error("dictation is disabled")]
    Disabled,
    #[error("this build has no local transcription engine")]
    LocalNotBuilt { model: LocalModelId },
    #[error("the dictation endpoint has no usable selected key")]
    KeyAbsent,
    #[error("the dictation secret could not be resolved")]
    Secret(#[source] ReadError),
    #[error("the dictation endpoint could not be prepared")]
    Endpoint(#[source] ProviderError),
}
impl std::fmt::Debug for PrepareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl From<ProviderError> for PrepareError {
    fn from(source: ProviderError) -> Self {
        match source {
            ProviderError::KeyAbsent => Self::KeyAbsent,
            source @ (ProviderError::Http(_)
            | ProviderError::Roots(_)
            | ProviderError::Address(_)
            | ProviderError::Header(_)
            | ProviderError::Form(_)
            | ProviderError::ResponseTooLarge
            | ProviderError::Count(_)
            | ProviderError::Decode(_)) => Self::Endpoint(source),
        }
    }
}

fn selection(
    saved: &SetDictation,
) -> Result<(SetDictation, vessel_proto::dictation::DictationSettings), PrepareError> {
    let saved = saved
        .clone()
        .over(SetDictation::default())
        .map_err(PrepareError::Configuration)?;
    let config = saved.resolve(KeyPresence::Absent);
    match config.switch {
        DictationSwitch::Off => return Err(PrepareError::Disabled),
        DictationSwitch::On => {}
        DictationSwitch::Unknown => {
            return Err(PrepareError::Configuration(
                DictationDecodeError::UnknownSwitch,
            ));
        }
    }
    match config.provider {
        DictationProvider::Local => {
            return Err(PrepareError::LocalNotBuilt {
                model: config.local_model,
            });
        }
        DictationProvider::Endpoint => {}
        DictationProvider::Unknown => {
            return Err(PrepareError::Configuration(
                DictationDecodeError::UnknownProvider,
            ));
        }
    }
    Ok((saved, config))
}

/// One factory per service: roots/read capacity survive individual start waiters.
pub(super) struct Factory {
    reader: Reader,
    roots: roots::Factory,
}
impl Factory {
    /// Share a synthetic loader counter across service recreation, never in product.
    #[cfg(test)]
    pub(super) fn with_root_probe(
        reader: Reader,
        probe: Arc<std::sync::atomic::AtomicUsize>,
    ) -> Self {
        Self {
            reader,
            roots: roots::Factory::refusing_once(probe),
        }
    }

    /// Share the admitted read gate while creating exactly one root cache.
    pub(super) fn new(reader: Reader) -> Self {
        Self {
            reader,
            roots: roots::Factory::default(),
        }
    }
    /// A settings candidate is metadata only; credentials are read on every start.
    pub(super) async fn prepare(
        &self,
        saved: &SetDictation,
        lease: Arc<Lease>,
        stopped: &CancellationToken,
    ) -> Result<Source, PrepareError> {
        let (saved, config) = selection(saved)?;
        let name = saved.key_name().ok_or(PrepareError::KeyAbsent)?;
        let key = self
            .reader
            .resolve(
                name,
                Admission::Preparing {
                    lease: Arc::clone(&lease),
                    stopped,
                },
            )
            .await
            .map_err(PrepareError::Secret)?
            .ok_or(PrepareError::KeyAbsent)?;
        let specification = Specification::new(
            &config.endpoint_base,
            config.endpoint_model,
            config.language,
            &key,
        )
        .map_err(PrepareError::from)?;
        specification
            .connect(&self.roots)
            .await
            .map(Source::Endpoint)
            .map_err(PrepareError::from)
    }
}

#[cfg(test)]
mod tests;
