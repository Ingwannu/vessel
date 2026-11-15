//! Dictation settings contract (ADR 0067), without node wiring, files or I/O.
//! Defaults and patch validation are pure; only the existing node secret owner
//! resolves the selected reference during persistence and stream preparation.

use serde::{Deserialize, Serialize};

use super::{
    DictationChange, DictationDecodeError, EndpointBase, EndpointModel, Language, LocalModelId,
    SecretReference,
};
use crate::name::SecretName;

/// `on` or `off`. `unknown` is a newer node's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DictationSwitch {
    /// Dictation may start.
    On,
    /// `start` fails `Disabled`. The saved value stays.
    Off,
    /// A newer node's value. Refused on write. `start` fails.
    #[serde(other)]
    Unknown,
}

/// Where the audio is transcribed. `unknown` is a newer node's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DictationProvider {
    /// This build has no engine. `start` fails `LocalNotBuilt`.
    Local,
    /// An OpenAI-compatible transcription endpoint.
    Endpoint,
    /// A newer node's value. Refused on write. `start` fails.
    #[serde(other)]
    Unknown,
}

/// Whether `start` can authenticate. Not a secret name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum KeyPresence {
    /// The named secret has a value.
    Present,
    /// No name was saved, or the name has no value.
    Absent,
}

/// What a reader sees. Defaults filled in. `key` is present or absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DictationSettings {
    /// Whether dictation may start.
    pub switch: DictationSwitch,
    /// Which provider `start` uses.
    pub provider: DictationProvider,
    /// Sent to the endpoint.
    pub language: Language,
    /// Named in `LocalNotBuilt` when the provider is local.
    pub local_model: LocalModelId,
    /// The endpoint's model.
    pub endpoint_model: EndpointModel,
    /// The endpoint's base URL.
    pub endpoint_base: EndpointBase,
    /// Whether the named secret has a value.
    pub key: KeyPresence,
}

impl DictationSettings {
    /// Every field is the default and no key is stored.
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl Default for DictationSettings {
    fn default() -> Self {
        SetDictation::default().resolve(KeyPresence::Absent)
    }
}

/// A change. Each field is optional; `unknown` and an unknown local
/// model are refused and nothing is written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SetDictation {
    /// `on` or `off`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::deserialize_change"
    )]
    pub switch: Option<DictationChange<DictationSwitch>>,
    /// `local` or `endpoint`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::deserialize_change"
    )]
    pub provider: Option<DictationChange<DictationProvider>>,
    /// A language code.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::deserialize_change"
    )]
    pub language: Option<DictationChange<Language>>,
    /// A catalog id.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::deserialize_change"
    )]
    pub local_model: Option<DictationChange<LocalModelId>>,
    /// The endpoint's model.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::deserialize_change"
    )]
    pub endpoint_model: Option<DictationChange<EndpointModel>>,
    /// The endpoint's base URL.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::deserialize_change"
    )]
    pub endpoint_base: Option<DictationChange<EndpointBase>>,
    /// The secret reference to set or explicitly clear, never its value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::deserialize_change"
    )]
    pub key: Option<DictationChange<SecretReference>>,
}

impl SetDictation {
    /// Refuse a value this build will not keep.
    ///
    /// # Errors
    /// The switch, the provider, or the local model is not one this build knows.
    pub fn check(&self) -> Result<(), DictationDecodeError> {
        if self.switch.as_ref().and_then(DictationChange::value) == Some(&DictationSwitch::Unknown)
        {
            return Err(DictationDecodeError::UnknownSwitch);
        }
        if self.provider.as_ref().and_then(DictationChange::value)
            == Some(&DictationProvider::Unknown)
        {
            return Err(DictationDecodeError::UnknownProvider);
        }
        if self
            .local_model
            .as_ref()
            .and_then(DictationChange::value)
            .is_some_and(|id| !id.known())
        {
            return Err(DictationDecodeError::UnknownLocalModel);
        }
        Ok(())
    }

    /// The selected name goes only to the existing node secret lookup.
    #[must_use]
    pub fn key_name(&self) -> Option<&SecretName> {
        self.key
            .as_ref()
            .and_then(DictationChange::value)
            .map(SecretReference::selected_name)
    }

    /// Apply a patch only when its effective provider accepts every selected field.
    ///
    /// # Errors
    /// Unknown choices or options for an inactive provider would otherwise be kept.
    pub fn over(self, base: Self) -> Result<Self, DictationDecodeError> {
        self.check()?;
        // A null provider resets its site even when local was already in force;
        // naming that same provider explicitly preserves the existing site.
        let changed = self.provider.as_ref().is_some_and(|change| match change {
            DictationChange::Set(provider) => *provider != base.provider_choice(),
            DictationChange::Reset => true,
        });
        let base = if changed {
            Self {
                switch: base.switch,
                ..Self::default()
            }
        } else {
            base
        };
        let merged = Self {
            switch: DictationChange::overlay(self.switch, base.switch),
            provider: DictationChange::overlay(self.provider, base.provider),
            language: DictationChange::overlay(self.language, base.language),
            local_model: DictationChange::overlay(self.local_model, base.local_model),
            endpoint_model: DictationChange::overlay(self.endpoint_model, base.endpoint_model),
            endpoint_base: DictationChange::overlay(self.endpoint_base, base.endpoint_base),
            key: DictationChange::overlay(self.key, base.key),
        };
        merged.check()?;
        merged.check_fields()?;
        Ok(merged)
    }

    fn provider_choice(&self) -> DictationProvider {
        self.provider
            .as_ref()
            .and_then(DictationChange::value)
            .copied()
            .unwrap_or(DictationProvider::Local)
    }

    fn check_fields(&self) -> Result<(), DictationDecodeError> {
        match self.provider_choice() {
            DictationProvider::Local => {
                if self.endpoint_model.is_some()
                    || self.endpoint_base.is_some()
                    || self.key.is_some()
                {
                    return Err(DictationDecodeError::EndpointFields);
                }
            }
            DictationProvider::Endpoint => {
                if self.local_model.is_some() {
                    return Err(DictationDecodeError::LocalFields);
                }
            }
            DictationProvider::Unknown => return Err(DictationDecodeError::UnknownProvider),
        }
        Ok(())
    }

    /// Fill defaults. `key` reports availability without exposing its name.
    #[must_use]
    pub fn resolve(&self, key: KeyPresence) -> DictationSettings {
        DictationSettings {
            switch: self
                .switch
                .as_ref()
                .and_then(DictationChange::value)
                .copied()
                .unwrap_or(DictationSwitch::On),
            provider: self.provider_choice(),
            language: self
                .language
                .as_ref()
                .and_then(DictationChange::value)
                .cloned()
                .unwrap_or_else(Language::english),
            local_model: self
                .local_model
                .as_ref()
                .and_then(DictationChange::value)
                .cloned()
                .unwrap_or_else(LocalModelId::default_model),
            endpoint_model: self
                .endpoint_model
                .as_ref()
                .and_then(DictationChange::value)
                .cloned()
                .unwrap_or_else(EndpointModel::whisper),
            endpoint_base: self
                .endpoint_base
                .as_ref()
                .and_then(DictationChange::value)
                .cloned()
                .unwrap_or_else(EndpointBase::openai),
            key,
        }
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
