//! Voice-only settings values and patches; the existing node settings owner persists them.
//!
//! These types open no provider, secret, model or session. Unknown reads remain
//! representable; edits validate before work. Wiring must preserve settings.json's
//! existing unknown-field set-aside instead of creating a second store.

mod stt;
mod tts;
mod values;

pub use stt::{SetVoiceStt, VoiceSttSettings};
pub use tts::{SetVoiceTts, VoiceTtsSettings};
pub use values::{LocalTtsModelId, TtsModel, TtsVoice, UnknownTtsModelId};

use crate::SecretName;
use crate::dictation::{
    DictationChange, DictationDecodeError, DictationSwitch, KeyPresence, SecretReference,
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Configuration errors contain the violated rule, never submitted values.
#[derive(Debug, thiserror::Error)]
pub enum VoiceSettingsError {
    /// Reuse the existing STT site's validation and preserve its cause.
    #[error("voice transcription settings are invalid")]
    Stt(#[source] DictationDecodeError),
    /// TTS identifiers reuse the existing bounded metadata edge.
    #[error("voice provider metadata is invalid")]
    Metadata(#[source] DictationDecodeError),
    /// Future switch values are readable, never newly selected.
    #[error("this node does not know that voice switch")]
    Switch,
    /// A future synthesis provider requires a newer node.
    #[error("this node does not know that voice synthesis provider")]
    Provider,
    /// A future model may survive read/downgrade but not a new selection.
    #[error("this node does not know that local synthesis model")]
    LocalModel,
    /// Local and endpoint configuration must not accumulate dormant keys.
    #[error("voice synthesis endpoint options require the endpoint provider")]
    EndpointFields,
    /// Endpoint sites cannot retain a local model override.
    #[error("a local synthesis model requires the local provider")]
    LocalFields,
}

/// Read settings expose presence only; no secret reference name or value.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct VoiceSettings {
    /// Voice is opt-in independently of dictation's default-On switch.
    pub switch: DictationSwitch,
    /// One independent recognition site, without its own enabled switch.
    pub stt: VoiceSttSettings,
    /// One independent synthesis site; local readiness remains separate.
    pub tts: VoiceTtsSettings,
}
impl Default for VoiceSettings {
    fn default() -> Self {
        SetVoice::default().resolve(KeyPresence::Absent, KeyPresence::Absent)
    }
}
impl fmt::Debug for VoiceSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceSettings")
            .field("switch", &self.switch)
            .finish_non_exhaustive()
    }
}

/// Optional fields distinguish omission, explicit reset and a selected value.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SetVoice {
    /// A null resets to Off, never dictation's On default.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub switch: Option<DictationChange<DictationSwitch>>,
    /// Missing preserves this site; null resets the entire site.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub stt: Option<DictationChange<SetVoiceStt>>,
    /// Synthesis changes never overwrite recognition settings.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::dictation::deserialize_change"
    )]
    pub tts: Option<DictationChange<SetVoiceTts>>,
}
impl fmt::Debug for SetVoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetVoice").finish_non_exhaustive()
    }
}
impl SetVoice {
    /// Apply only changed sites, so an unrelated switch edit preserves future read choices.
    ///
    /// # Errors
    /// Unknown selected choices or provider-inactive options are refused atomically.
    pub fn over(self, base: Self) -> Result<Self, VoiceSettingsError> {
        if self.switch.as_ref().and_then(DictationChange::value) == Some(&DictationSwitch::Unknown)
        {
            return Err(VoiceSettingsError::Switch);
        }
        Ok(Self {
            switch: DictationChange::overlay(self.switch, base.switch),
            stt: overlay_site(self.stt, base.stt, SetVoiceStt::over)?,
            tts: overlay_site(self.tts, base.tts, SetVoiceTts::over)?,
        })
    }
    /// Fill defaults from the existing site owners, without resolving a credential.
    #[must_use]
    pub fn resolve(&self, stt_key: KeyPresence, tts_key: KeyPresence) -> VoiceSettings {
        VoiceSettings {
            switch: self
                .switch
                .as_ref()
                .and_then(DictationChange::value)
                .copied()
                .unwrap_or(DictationSwitch::Off),
            stt: self
                .stt
                .as_ref()
                .and_then(DictationChange::value)
                .map_or_else(
                    || SetVoiceStt::default().resolve(stt_key),
                    |site| site.resolve(stt_key),
                ),
            tts: self
                .tts
                .as_ref()
                .and_then(DictationChange::value)
                .map_or_else(
                    || SetVoiceTts::default().resolve(tts_key),
                    |site| site.resolve(tts_key),
                ),
        }
    }
    /// Only the existing secret owner may use this name to compute presence/connect.
    #[must_use]
    pub fn stt_key_name(&self) -> Option<&SecretName> {
        self.stt
            .as_ref()
            .and_then(DictationChange::value)
            .and_then(SetVoiceStt::key_name)
    }
    /// Synthesis keys are independent from recognition keys; there is no environment fallback.
    #[must_use]
    pub fn tts_key_name(&self) -> Option<&SecretName> {
        self.tts
            .as_ref()
            .and_then(DictationChange::value)
            .and_then(SetVoiceTts::key_name)
    }
}
fn key_name(change: &Option<DictationChange<SecretReference>>) -> Option<&SecretName> {
    change
        .as_ref()
        .and_then(DictationChange::value)
        .map(SecretReference::selected_name)
}
fn overlay_site<T: Default>(
    change: Option<DictationChange<T>>,
    saved: Option<DictationChange<T>>,
    merge: impl FnOnce(T, T) -> Result<T, VoiceSettingsError>,
) -> Result<Option<DictationChange<T>>, VoiceSettingsError> {
    match change {
        None => Ok(saved),
        Some(DictationChange::Reset) => Ok(None),
        Some(DictationChange::Set(change)) => {
            let base = match saved {
                Some(DictationChange::Set(base)) => base,
                Some(DictationChange::Reset) | None => T::default(),
            };
            merge(change, base).map(|site| Some(DictationChange::Set(site)))
        }
    }
}

#[cfg(test)]
mod tests;
