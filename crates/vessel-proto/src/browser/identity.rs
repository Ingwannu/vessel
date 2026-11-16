//! A browser id must match the reference grammar, not merely parse as a UUID.
//!
//! The legacy timestamp form is accepted for compatibility. Node-created ids
//! always use canonical `UUIDv4`; element refs retain their exact spelling so a
//! caller cannot turn an unrelated decimal spelling into a current snapshot ref.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use uuid::{Uuid, Variant, Version};

/// The reference timestamp-id alternative requires this many decimal digits.
const LEGACY_TIMESTAMP_MIN_DIGITS: usize = 13;

/// Malformed input is reported without reflecting untrusted page or tool text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidBrowserIdentity {
    /// The browser id matches neither of the reference's two formats.
    #[error("browser id must be a canonical UUIDv4 or a timestamp-hex identifier")]
    Browser,
    /// Refs are assigned only by a snapshot, in its @e<decimal> namespace.
    #[error("element reference must be @e followed by decimal digits")]
    Element,
}

/// A node browser target, independent of a pane or CDP target identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct BrowserId(Arc<str>);

impl BrowserId {
    /// A live engine mints this only after its target has been created.
    #[must_use]
    pub fn random() -> Self {
        Self(Arc::from(Uuid::new_v4().hyphenated().to_string()))
    }

    /// Encoding and layout adapters need the validated identity, not its grammar.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for BrowserId {
    type Err = InvalidBrowserIdentity;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if let Ok(uuid) = Uuid::parse_str(text)
            && uuid.get_version() == Some(Version::Random)
            && uuid.get_variant() == Variant::RFC4122
            && uuid.hyphenated().to_string().eq_ignore_ascii_case(text)
        {
            return Ok(Self(Arc::from(uuid.hyphenated().to_string())));
        }
        let (timestamp, suffix) = text
            .split_once('-')
            .ok_or(InvalidBrowserIdentity::Browser)?;
        if timestamp.len() < LEGACY_TIMESTAMP_MIN_DIGITS
            || !timestamp.bytes().all(|byte| byte.is_ascii_digit())
            || suffix.is_empty()
            || !suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(InvalidBrowserIdentity::Browser);
        }
        Ok(Self(Arc::from(text.to_ascii_lowercase())))
    }
}

impl fmt::Display for BrowserId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

crate::key::serde_via_text!(BrowserId);

/// A snapshot-local element reference; ownership/generation is checked by the engine.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ElementRef(Arc<str>);

impl ElementRef {
    /// Keep the spelling used as a key in the current snapshot's reference map.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ElementRef {
    type Err = InvalidBrowserIdentity;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let digits = text
            .strip_prefix("@e")
            .ok_or(InvalidBrowserIdentity::Element)?;
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(InvalidBrowserIdentity::Element);
        }
        Ok(Self(Arc::from(text)))
    }
}

impl fmt::Display for ElementRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

crate::key::serde_via_text!(ElementRef);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_parser_extensions_are_not_the_browser_id_grammar() {
        let uuid = Uuid::new_v4();
        let canonical = uuid.hyphenated().to_string();
        assert_eq!(canonical.parse::<BrowserId>().unwrap().as_str(), canonical);
        assert!(canonical.to_uppercase().parse::<BrowserId>().is_ok());
        for invalid in [
            uuid.simple().to_string(),
            uuid.braced().to_string(),
            uuid.urn().to_string(),
            format!(" {canonical}"),
            Uuid::nil().hyphenated().to_string(),
            "00000000-0000-4000-0000-000000000000".to_owned(),
            "00000000-0000-4000-c000-000000000000".to_owned(),
            "00000000-0000-4000-e000-000000000000".to_owned(),
        ] {
            assert!(invalid.parse::<BrowserId>().is_err());
            assert!(
                serde_json::from_str::<BrowserId>(&serde_json::to_string(&invalid).unwrap())
                    .is_err()
            );
        }
    }

    #[test]
    fn legacy_ids_preserve_reference_boundaries_and_normalize_hex_case() {
        let minimum = "1".repeat(LEGACY_TIMESTAMP_MIN_DIGITS);
        let short = "1".repeat(LEGACY_TIMESTAMP_MIN_DIGITS.saturating_sub(1));
        let accepted: BrowserId = format!("{minimum}-AbCd").parse().unwrap();
        assert_eq!(accepted.as_str(), format!("{minimum}-abcd"));
        for bad in [
            format!("{short}-a"),
            format!("{minimum}-"),
            format!("{minimum}-a-b"),
            format!("{minimum}-g"),
        ] {
            assert!(bad.parse::<BrowserId>().is_err());
        }
        let encoded = serde_json::to_string(&accepted).unwrap();
        assert_eq!(
            serde_json::from_str::<BrowserId>(&encoded).unwrap(),
            accepted
        );
    }

    #[test]
    fn element_refs_are_exact_ascii_snapshot_keys() {
        for good in ["@e0", "@e1", "@e0001"] {
            assert_eq!(good.parse::<ElementRef>().unwrap().as_str(), good);
        }
        for bad in ["@e", "@e-1", "@e１", "@E1", " @e1", "@e1\n"] {
            assert!(bad.parse::<ElementRef>().is_err());
        }
        assert_ne!(
            "@e1".parse::<ElementRef>().unwrap(),
            "@e01".parse::<ElementRef>().unwrap()
        );
    }

    #[test]
    fn bad_identity_errors_never_echo_the_input() {
        let secret = "credential-canary";
        let error = secret.parse::<BrowserId>().unwrap_err();
        assert!(!error.to_string().contains(secret));
        assert!(!format!("{error:?}").contains(secret));
    }
}
