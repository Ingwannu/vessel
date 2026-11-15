//! Validated app-supplied legacy inputs; never reads a preference or account file.
//!
//! Exact opaque keys are not trimmed or mapped to a provider's option dialect.
//! A consumer must resolve them through an actual reported catalogue. Bounds
//! include duplicate source entries; deduplication belongs to the pure planner.

use super::{InvalidLaunchValue, LaunchChoice, LaunchProfiles, codec};
use serde::{Deserialize, Deserializer, Serialize, de};
use std::{fmt, str::FromStr, sync::Arc};

#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
struct Key(Arc<str>);
impl FromStr for Key {
    type Err = InvalidLaunchValue;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.len() > LaunchChoice::MAX_ID_BYTES
            || value.chars().any(char::is_control)
        {
            return Err(InvalidLaunchValue::LegacyKey);
        }
        Ok(Self(Arc::from(value)))
    }
}
impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LegacyKey(..)")
    }
}
impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let value: std::borrow::Cow<'de, str> = codec::decode(input)?;
        value.parse().map_err(de::Error::custom)
    }
}
macro_rules! key {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub struct $name(Key);
        impl FromStr for $name {
            type Err = InvalidLaunchValue;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                value.parse().map(Self)
            }
        }
        impl $name {
            /// Exact public source key; never a command, account or native mapping.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0.0
            }
        }
    };
}
key!(
    LegacyProviderKey,
    "An opaque validated provider identity from local compatibility input."
);
key!(
    LegacyModelKey,
    "An opaque validated model identity; only actual catalogue ids may resolve it."
);

/// One validated local favorite, without any credential or account override.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LegacyFavorite {
    provider: LegacyProviderKey,
    model: LegacyModelKey,
}
impl LegacyFavorite {
    /// Pair exact typed public identities, rather than concatenate ambiguous text.
    #[must_use]
    pub const fn new(provider: LegacyProviderKey, model: LegacyModelKey) -> Self {
        Self { provider, model }
    }
    /// Provider key the app's actual catalogue loader must resolve.
    #[must_use]
    pub const fn provider(&self) -> &LegacyProviderKey {
        &self.provider
    }
    /// Model key; no implicit ACP/native id translation is authorized.
    #[must_use]
    pub const fn model(&self) -> &LegacyModelKey {
        &self.model
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EncodedFavorite {
    provider: LegacyProviderKey,
    model: LegacyModelKey,
}
impl<'de> Deserialize<'de> for LegacyFavorite {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let value: EncodedFavorite = codec::decode(input)?;
        Ok(Self::new(value.provider, value.model))
    }
}

/// Finite ordered compatibility inputs; duplicate pairs are intentionally retained.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(as = "Vec<LegacyFavorite>"))]
pub struct LegacyFavorites(Arc<[LegacyFavorite]>);
impl LegacyFavorites {
    /// Reuse the host list budget for one migration attempt's input.
    pub const MAX_ENTRIES: usize = LaunchProfiles::MAX_PROFILES;
    /// Validated source entries in order, before planner deduplication.
    #[must_use]
    pub fn as_slice(&self) -> &[LegacyFavorite] {
        &self.0
    }
}
impl TryFrom<Vec<LegacyFavorite>> for LegacyFavorites {
    type Error = InvalidLaunchValue;
    fn try_from(values: Vec<LegacyFavorite>) -> Result<Self, Self::Error> {
        if values.len() > Self::MAX_ENTRIES {
            return Err(InvalidLaunchValue::LegacyFavorites);
        }
        Ok(Self(Arc::from(values)))
    }
}
impl<'de> Deserialize<'de> for LegacyFavorites {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let values = codec::bounded::<D, LegacyFavorite, { Self::MAX_ENTRIES }>(input)?;
        Self::try_from(values).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LaunchProfileId;

    fn favorite(provider: &str, model: &str) -> LegacyFavorite {
        LegacyFavorite::new(provider.parse().unwrap(), model.parse().unwrap())
    }

    #[test]
    fn exact_keys_are_bounded_and_do_not_silently_normalize() {
        let exact = "p".repeat(LaunchChoice::MAX_ID_BYTES);
        assert_eq!(exact.parse::<LegacyProviderKey>().unwrap().as_str(), exact);
        assert!(format!("{exact}x").parse::<LegacyProviderKey>().is_err());
        assert!("".parse::<LegacyModelKey>().is_err());
        assert!("model\nkey".parse::<LegacyModelKey>().is_err());
        assert_eq!(
            " model ".parse::<LegacyModelKey>().unwrap().as_str(),
            " model "
        );
        let invalid = r#"{"provider":"p","model":"m","private_unknown":"not echoed"}"#;
        let error = serde_json::from_str::<LegacyFavorite>(invalid).unwrap_err();
        assert!(!format!("{error:?}").contains("private_unknown"));
    }

    #[test]
    fn canonical_namespace_id_is_stable_and_the_pair_encoding_is_unambiguous() {
        let pair = favorite("fixture-provider", "model-a");
        let id = LaunchProfileId::for_legacy_favorite(&pair).unwrap();
        assert_eq!(id.to_string(), "13e33a2d-50cb-5429-aee4-c823d045bcb2");
        assert_eq!(id, LaunchProfileId::for_legacy_favorite(&pair).unwrap());
        assert_ne!(
            LaunchProfileId::for_legacy_favorite(&favorite("a:b", "c")).unwrap(),
            LaunchProfileId::for_legacy_favorite(&favorite("a", "b:c")).unwrap()
        );
        let parsed: LegacyFavorite =
            serde_json::from_str(&serde_json::to_string(&pair).unwrap()).unwrap();
        assert_eq!(LaunchProfileId::for_legacy_favorite(&parsed).unwrap(), id);
    }

    #[test]
    fn source_duplicates_are_preserved_with_a_whole_input_cap_before_planning() {
        let values = vec![favorite("provider", "model"); LegacyFavorites::MAX_ENTRIES];
        let full = LegacyFavorites::try_from(values.clone()).unwrap();
        let decoded: LegacyFavorites =
            serde_json::from_str(&serde_json::to_string(&full).unwrap()).unwrap();
        assert_eq!(decoded.as_slice(), values);
        let mut extra = values;
        extra.push(favorite("provider", "model"));
        assert!(LegacyFavorites::try_from(extra.clone()).is_err());
        assert!(
            serde_json::from_str::<LegacyFavorites>(&serde_json::to_string(&extra).unwrap())
                .is_err()
        );
    }
}
