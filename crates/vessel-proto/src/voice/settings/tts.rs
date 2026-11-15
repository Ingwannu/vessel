//! Independent synthesis-site overlay over the existing presence/reset primitive.

use super::{LocalTtsModelId, TtsModel, TtsVoice, VoiceSettingsError, key_name};
use crate::{
    SecretName,
    dictation::{DictationChange, DictationProvider, EndpointBase, KeyPresence, SecretReference},
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Filled synthesis defaults; readiness/model installation is deliberately separate.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct VoiceTtsSettings {
    /// Local is the default, never an implicit endpoint fallback.
    pub provider: DictationProvider,
    /// Reference catalogue metadata does not approve its files or runtime.
    pub local_model: LocalTtsModelId,
    /// Bounded endpoint model, tts-1 by default.
    pub endpoint_model: TtsModel,
    /// Bounded voice identifier, alloy by default.
    pub voice: TtsVoice,
    /// Existing TLS/loopback validated URL primitive.
    pub endpoint_base: EndpointBase,
    /// Presence only, never a name/value in the read response.
    pub key: KeyPresence,
}
impl fmt::Debug for VoiceTtsSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceTtsSettings").finish_non_exhaustive()
    }
}
/// Site-specific writes reuse the checked omission/null/Set visitor.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SetVoiceTts {
    /// Changing/resetting the provider clears old site options.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub provider: Option<DictationChange<DictationProvider>>,
    /// Metadata only; selection never downloads or loads a model.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub local_model: Option<DictationChange<LocalTtsModelId>>,
    /// A bounded custom synthesis model identifier.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub endpoint_model: Option<DictationChange<TtsModel>>,
    /// A distinct bounded speaker/voice identifier.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub voice: Option<DictationChange<TtsVoice>>,
    /// A validated endpoint base; no query/userinfo credentials.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub endpoint_base: Option<DictationChange<EndpointBase>>,
    /// Existing secret-reference decoding/redaction, not a raw key.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub key: Option<DictationChange<SecretReference>>,
}
impl fmt::Debug for SetVoiceTts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetVoiceTts").finish_non_exhaustive()
    }
}
impl SetVoiceTts {
    /// Refuse unknown incoming choices without altering either original value.
    ///
    /// # Errors
    /// A future provider/model cannot be newly applied by this build.
    pub fn check(&self) -> Result<(), VoiceSettingsError> {
        if self.provider.as_ref().and_then(DictationChange::value)
            == Some(&DictationProvider::Unknown)
        {
            return Err(VoiceSettingsError::Provider);
        }
        if self
            .local_model
            .as_ref()
            .and_then(DictationChange::value)
            .is_some_and(|id| !id.known())
        {
            return Err(VoiceSettingsError::LocalModel);
        }
        Ok(())
    }
    /// An explicit provider reset clears the site, even when already local.
    ///
    /// # Errors
    /// Unknown choices or inactive-provider fields are refused atomically.
    pub fn over(self, base: Self) -> Result<Self, VoiceSettingsError> {
        self.check()?;
        let changed = self.provider.as_ref().is_some_and(|change| match change {
            DictationChange::Set(provider) => *provider != base.provider_choice(),
            DictationChange::Reset => true,
        });
        let base = if changed { Self::default() } else { base };
        let merged = Self {
            provider: DictationChange::overlay(self.provider, base.provider),
            local_model: DictationChange::overlay(self.local_model, base.local_model),
            endpoint_model: DictationChange::overlay(self.endpoint_model, base.endpoint_model),
            voice: DictationChange::overlay(self.voice, base.voice),
            endpoint_base: DictationChange::overlay(self.endpoint_base, base.endpoint_base),
            key: DictationChange::overlay(self.key, base.key),
        };
        merged.check()?;
        match merged.provider_choice() {
            DictationProvider::Local => {
                if merged.endpoint_model.is_some()
                    || merged.voice.is_some()
                    || merged.endpoint_base.is_some()
                    || merged.key.is_some()
                {
                    return Err(VoiceSettingsError::EndpointFields);
                }
            }
            DictationProvider::Endpoint => {
                if merged.local_model.is_some() {
                    return Err(VoiceSettingsError::LocalFields);
                }
            }
            DictationProvider::Unknown => return Err(VoiceSettingsError::Provider),
        }
        Ok(merged)
    }
    fn provider_choice(&self) -> DictationProvider {
        self.provider
            .as_ref()
            .and_then(DictationChange::value)
            .copied()
            .unwrap_or(DictationProvider::Local)
    }
    /// Defaults are values only; no key lookup, model download or provider connection.
    #[must_use]
    pub fn resolve(&self, key: KeyPresence) -> VoiceTtsSettings {
        VoiceTtsSettings {
            provider: self.provider_choice(),
            local_model: self
                .local_model
                .as_ref()
                .and_then(DictationChange::value)
                .cloned()
                .unwrap_or_default(),
            endpoint_model: self
                .endpoint_model
                .as_ref()
                .and_then(DictationChange::value)
                .cloned()
                .unwrap_or_default(),
            voice: self
                .voice
                .as_ref()
                .and_then(DictationChange::value)
                .cloned()
                .unwrap_or_default(),
            endpoint_base: self
                .endpoint_base
                .as_ref()
                .and_then(DictationChange::value)
                .cloned()
                .unwrap_or_else(EndpointBase::openai),
            key,
        }
    }
    /// Only the existing node secret owner may use this selected name.
    #[must_use]
    pub fn key_name(&self) -> Option<&SecretName> {
        key_name(&self.key)
    }
}
