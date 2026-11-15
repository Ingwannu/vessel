//! Typed mutable provider overrides. Storage must preserve unknown fields before
//! writing a later file; this decoder alone is not a persistence implementation.

use super::{
    CommandEdit, DisallowedTools, Enabled, Models, ProviderEnvironment, ProviderId, ProviderKind,
};
use crate::agents::AgentLabel;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Maximum configured or registered providers on one node.
pub const MAX_PROVIDERS: usize = 128;

/// Per-provider override, distinct from a named session launch profile.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProviderOverride {
    /// None allows a plugin override to precede registration.
    pub extends: Option<ProviderKind>,
    /// Derived providers must declare a human label.
    pub label: Option<AgentLabel>,
    /// Absence inherits manifest opt-in state for built-ins.
    pub enabled: Option<Enabled>,
    /// Absence preserves the base command edit.
    pub command: Option<CommandEdit>,
    /// Shallow environment overlay, with values hidden from Debug.
    #[serde(default)]
    pub env: ProviderEnvironment,
    /// Concatenated with the implementation's inherited policy.
    #[serde(default)]
    pub disallowed_tools: DisallowedTools,
    /// Nonempty list replaces model discovery; empty behaves as no replacement.
    pub models: Option<Models>,
    /// Additions merge over runtime or replacement models.
    #[serde(default)]
    pub additional_models: Models,
}

/// One validated config map; duplicate JSON keys are refused rather than lost.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(transparent)]
pub struct ProviderOverrides(BTreeMap<ProviderId, ProviderOverride>);

impl ProviderOverrides {
    /// Bound input before building any provider client.
    ///
    /// # Errors
    /// `ConfigurationError`::`TooMany` refuses an oversized map as a whole.
    pub fn new(
        entries: BTreeMap<ProviderId, ProviderOverride>,
    ) -> Result<Self, ConfigurationError> {
        if entries.len() > MAX_PROVIDERS {
            return Err(ConfigurationError::TooMany);
        }
        Ok(Self(entries))
    }

    /// Definitions by effective identity; values must not be published to readers.
    #[must_use]
    pub const fn entries(&self) -> &BTreeMap<ProviderId, ProviderOverride> {
        &self.0
    }
}

/// Refusal of malformed or ambiguous provider override maps.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigurationError {
    /// Finite node resource bound.
    #[error("provider registry exceeds {MAX_PROVIDERS} entries")]
    TooMany,
    /// Repeated keys must not silently discard configuration.
    #[error("provider override repeats an identifier")]
    Duplicate,
}

impl<'de> Deserialize<'de> for ProviderOverrides {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> serde::de::Visitor<'de> for Entries {
            type Value = ProviderOverrides;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "at most {MAX_PROVIDERS} unique provider entries")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut entries = BTreeMap::new();
                while let Some((id, entry)) = map.next_entry()? {
                    if entries.len() == MAX_PROVIDERS {
                        return Err(serde::de::Error::custom(ConfigurationError::TooMany));
                    }
                    if entries.insert(id, entry).is_some() {
                        return Err(serde::de::Error::custom(ConfigurationError::Duplicate));
                    }
                }
                ProviderOverrides::new(entries).map_err(serde::de::Error::custom)
            }
        }
        de.deserialize_map(Entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_overrides_fail_and_future_override_fields_are_tolerated() {
        assert!(serde_json::from_str::<ProviderOverrides>(r#"{"new":{},"new":{}}"#).is_err());
        let parsed: ProviderOverrides = serde_json::from_str(
            r#"{"new":{"future_option":"retained by storage, not by this decoder"}}"#,
        )
        .unwrap();
        assert_eq!(parsed.entries().len(), 1);
        assert_eq!(
            serde_json::from_str::<ProviderOverrides>(&serde_json::to_string(&parsed).unwrap())
                .unwrap(),
            parsed
        );
    }
}
