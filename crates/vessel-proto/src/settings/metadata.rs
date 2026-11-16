//! Bounded generated-name routing values (ADR 0062). This module owns
//! shape and count validation, not node settings or model availability.

use crate::{ModelId, ModelRouteName};
use serde::de::{Error as _, IgnoredAny, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::{fmt, marker::PhantomData};

/// The cost bound on a stored manual candidate list, including duplicates.
pub const MAX_METADATA_PROVIDERS: usize = 8;

/// How a generated name is routed (ADR 0062). Empty [`providers`](Self::providers)
/// is automatic, the default, and what an older `settings.json` becomes.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "RawMetadataSettings")]
pub struct MetadataSettings {
    /// Manual routes, in the stored order. At most
    /// [`MAX_METADATA_PROVIDERS`]. Empty is automatic: nothing is appended.
    providers: Vec<MetadataProvider>,
}

impl MetadataSettings {
    /// Build a bounded list, preserving the stored manual order.
    /// This keeps wire decoding and programmatic callers on one rule.
    ///
    /// # Errors
    /// [`InvalidMetadata`] if more than [`MAX_METADATA_PROVIDERS`] entries were supplied,
    /// including repeated entries.
    pub fn new(providers: Vec<MetadataProvider>) -> Result<Self, InvalidMetadata> {
        if providers.len() > MAX_METADATA_PROVIDERS {
            return Err(InvalidMetadata);
        }
        Ok(Self { providers })
    }

    /// The manual candidates in order; an empty slice means automatic.
    #[must_use]
    pub fn providers(&self) -> &[MetadataProvider] {
        &self.providers
    }

    /// Whether generation picks a route itself.
    #[must_use]
    pub fn is_automatic(&self) -> bool {
        self.providers.is_empty()
    }

    /// Decode a retained view, ignoring fields a later file/reply may add.
    /// The ordinary Deserialize implementation remains the strict wire patch.
    /// No unknown content is retained here; the store owns set-aside copies.
    ///
    /// # Errors
    /// A known field is malformed, or the shared provider count is exceeded.
    pub fn deserialize_view<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = MetadataView::deserialize(deserializer)?;
        Self::new(
            raw.providers
                .into_iter()
                .map(MetadataProvider::from)
                .collect(),
        )
        .map_err(D::Error::custom)
    }

    /// Decode an optional retained view for replies and files, keeping `None`
    /// distinct from an available automatic selection. Patch decoding stays strict.
    ///
    /// # Errors
    /// A known field is malformed or the shared candidate bound is exceeded.
    pub fn deserialize_optional_view<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Self>, D::Error> {
        Option::<RetainedMetadata>::deserialize(deserializer).map(|view| view.map(|value| value.0))
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct RetainedMetadata(
    #[serde(deserialize_with = "MetadataSettings::deserialize_view")] MetadataSettings,
);

/// A provider list past its cost bound; the raw entries are never echoed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("metadata names more than {MAX_METADATA_PROVIDERS} providers")]
pub struct InvalidMetadata;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMetadataSettings {
    #[serde(default, deserialize_with = "deserialize_providers")]
    providers: Vec<MetadataProvider>,
}

// Keep allocation bounded too: an oversized wire list must not allocate
// every candidate before the constructor can reject its length.
fn deserialize_providers<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Vec<T>, D::Error> {
    struct Providers<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for Providers<T> {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                formatter,
                "at most {MAX_METADATA_PROVIDERS} metadata providers"
            )
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            let mut providers = Vec::with_capacity(MAX_METADATA_PROVIDERS);
            while providers.len() < MAX_METADATA_PROVIDERS {
                match sequence.next_element()? {
                    Some(provider) => providers.push(provider),
                    None => return Ok(providers),
                }
            }
            if sequence.next_element::<IgnoredAny>()?.is_some() {
                return Err(A::Error::custom(InvalidMetadata));
            }
            Ok(providers)
        }
    }
    deserializer.deserialize_seq(Providers(PhantomData))
}

impl TryFrom<RawMetadataSettings> for MetadataSettings {
    type Error = InvalidMetadata;

    fn try_from(raw: RawMetadataSettings) -> Result<Self, Self::Error> {
        Self::new(raw.providers)
    }
}

/// One manual route for generated names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct MetadataProvider {
    /// A route in `models.toml`.
    pub route: ModelRouteName,
    /// That route's model or its `small_model`. Absent prefers the small
    /// model when the route has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelId>,
    /// How hard that model works. Absent is the provider's default, except
    /// the automatic `gpt-5.4-mini` match, which asks for low.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<MetadataEffort>,
}

// Files and read-only views tolerate additions; writes use the strict type.
#[derive(Deserialize)]
struct MetadataView {
    #[serde(default, deserialize_with = "deserialize_providers")]
    providers: Vec<ProviderView>,
}

#[derive(Deserialize)]
struct ProviderView {
    route: ModelRouteName,
    #[serde(default)]
    model: Option<ModelId>,
    #[serde(default)]
    effort: Option<MetadataEffort>,
}

impl From<ProviderView> for MetadataProvider {
    fn from(value: ProviderView) -> Self {
        Self {
            route: value.route,
            model: value.model,
            effort: value.effort,
        }
    }
}

/// How hard a generated-name call asks the model to work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MetadataEffort {
    /// Barely.
    Minimal,
    /// Low.
    Low,
    /// Medium.
    Medium,
    /// High.
    High,
    /// The most the model offers.
    Max,
    /// A level a newer node named. Refused on write.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(route: &str) -> MetadataProvider {
        MetadataProvider {
            route: route.parse().unwrap(),
            model: None,
            effort: None,
        }
    }

    #[test]
    fn automatic_is_the_empty_list_and_missing_providers_default_to_it() {
        let automatic = MetadataSettings::default();
        assert!(automatic.is_automatic());
        assert!(automatic.providers().is_empty());
        assert_eq!(
            serde_json::from_str::<MetadataSettings>("{}").unwrap(),
            automatic
        );
        assert_eq!(
            serde_json::to_string(&automatic).unwrap(),
            r#"{"providers":[]}"#
        );
    }

    #[test]
    fn manual_order_and_duplicates_survive_a_round_trip() {
        let list = vec![provider("later"), provider("first"), provider("later")];
        let settings = MetadataSettings::new(list.clone()).unwrap();
        assert!(!settings.is_automatic());
        assert_eq!(settings.providers(), list);
        let encoded = serde_json::to_string(&settings).unwrap();
        let read: MetadataSettings = serde_json::from_str(&encoded).unwrap();
        assert_eq!(read, settings);
    }

    #[test]
    fn constructor_and_decoder_share_the_exact_candidate_bound() {
        let exact = MetadataSettings::new(vec![provider("fast"); MAX_METADATA_PROVIDERS]).unwrap();
        assert_eq!(exact.providers().len(), MAX_METADATA_PROVIDERS);
        assert_eq!(
            serde_json::from_str::<MetadataSettings>(&serde_json::to_string(&exact).unwrap())
                .unwrap(),
            exact
        );
        let over = vec![provider("fast"); MAX_METADATA_PROVIDERS + 1];
        let error = MetadataSettings::new(over.clone()).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("metadata names more than {MAX_METADATA_PROVIDERS} providers")
        );
        let raw = serde_json::json!({ "providers": over });
        let error = serde_json::from_value::<MetadataSettings>(raw).unwrap_err();
        assert!(
            error
                .to_string()
                .contains(&format!("{MAX_METADATA_PROVIDERS} providers"))
        );
    }

    #[test]
    fn known_preferences_round_trip_and_future_effort_is_typed_unknown() {
        let raw = r#"{"route":"fast","model":"small","effort":"low"}"#;
        let value: MetadataProvider = serde_json::from_str(raw).unwrap();
        assert_eq!(value.route.as_str(), "fast");
        assert_eq!(value.model.as_ref().unwrap().as_str(), "small");
        assert_eq!(value.effort, Some(MetadataEffort::Low));
        assert_eq!(serde_json::to_string(&value).unwrap(), raw);
        assert_eq!(
            serde_json::from_str::<MetadataEffort>(r#""future""#).unwrap(),
            MetadataEffort::Unknown
        );
    }

    #[test]
    fn misspelled_settings_and_candidate_keys_are_refused() {
        for raw in [
            r#"{"provider":[]}"#,
            r#"{"providers":[{"route":"fast","thinking":"low"}]}"#,
        ] {
            assert!(serde_json::from_str::<MetadataSettings>(raw).is_err());
        }
    }

    #[test]
    fn an_extra_element_is_refused_before_decoding_another_provider() {
        let mut entries = vec![serde_json::json!({"route":"fast"}); MAX_METADATA_PROVIDERS];
        entries.push(serde_json::json!("not a provider"));
        let error =
            serde_json::from_value::<MetadataSettings>(serde_json::json!({"providers":entries}))
                .unwrap_err();
        assert!(
            error
                .to_string()
                .contains(&format!("{MAX_METADATA_PROVIDERS} providers"))
        );
    }
}

#[cfg(test)]
mod view_tests;
