//! The switch-free voice recognition site reuses dictation's validated site policy.

use super::{VoiceSettingsError, key_name};
use crate::{
    SecretName,
    dictation::{
        DictationChange, DictationProvider, DictationSettings, EndpointBase, EndpointModel,
        KeyPresence, Language, LocalModelId, SecretReference, SetDictation,
    },
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A recognition site has no enabled switch of its own; VoiceSettings owns it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct VoiceSttSettings {
    /// Local or an OpenAI-compatible endpoint; future values remain readable.
    pub provider: DictationProvider,
    /// Recognition language, English by default.
    pub language: Language,
    /// Local catalogue metadata is not a model installation/readiness claim.
    pub local_model: LocalModelId,
    /// Endpoint recognition model, whisper-1 by default.
    pub endpoint_model: EndpointModel,
    /// Existing validated TLS/loopback address type.
    pub endpoint_base: EndpointBase,
    /// Presence only, never the stored reference name.
    pub key: KeyPresence,
}
impl fmt::Debug for VoiceSttSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceSttSettings").finish_non_exhaustive()
    }
}
/// The existing STT-site options, with no independent dictation/voice switch.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SetVoiceStt {
    /// A provider change/reset clears previous provider-specific options.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub provider: Option<DictationChange<DictationProvider>>,
    /// Missing preserves, null resets to English.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub language: Option<DictationChange<Language>>,
    /// A known local STT catalogue identifier, without model I/O.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub local_model: Option<DictationChange<LocalModelId>>,
    /// A bounded custom endpoint model.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub endpoint_model: Option<DictationChange<EndpointModel>>,
    /// A validated endpoint base, never a credential URL.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub endpoint_base: Option<DictationChange<EndpointBase>>,
    /// The existing redacted reference edge; not an API key value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub key: Option<DictationChange<SecretReference>>,
}
impl fmt::Debug for SetVoiceStt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetVoiceStt").finish_non_exhaustive()
    }
}
impl SetVoiceStt {
    /// Reuse one STT provider/reset/inactive-field rule instead of copying it.
    ///
    /// # Errors
    /// Existing STT configuration validation is preserved with its typed cause.
    pub fn over(self, base: Self) -> Result<Self, VoiceSettingsError> {
        self.into_dictation()
            .over(base.into_dictation())
            .map(Self::from_dictation)
            .map_err(VoiceSettingsError::Stt)
    }
    /// Only bounded metadata is projected; no PCM or secret value is cloned.
    #[must_use]
    pub fn resolve(&self, key: KeyPresence) -> VoiceSttSettings {
        let DictationSettings {
            switch: _,
            provider,
            language,
            local_model,
            endpoint_model,
            endpoint_base,
            key,
        } = self.clone().into_dictation().resolve(key);
        VoiceSttSettings {
            provider,
            language,
            local_model,
            endpoint_model,
            endpoint_base,
            key,
        }
    }
    /// Used only by the existing secret owner, not a read response.
    #[must_use]
    pub fn key_name(&self) -> Option<&SecretName> {
        key_name(&self.key)
    }
    fn into_dictation(self) -> SetDictation {
        SetDictation {
            switch: None,
            provider: self.provider,
            language: self.language,
            local_model: self.local_model,
            endpoint_model: self.endpoint_model,
            endpoint_base: self.endpoint_base,
            key: self.key,
        }
    }
    fn from_dictation(value: SetDictation) -> Self {
        Self {
            provider: value.provider,
            language: value.language,
            local_model: value.local_model,
            endpoint_model: value.endpoint_model,
            endpoint_base: value.endpoint_base,
            key: value.key,
        }
    }
}
