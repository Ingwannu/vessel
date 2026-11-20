//! A plugin id, checked once at registration.
//!
//! Paseo's id is `^[a-z][a-z0-9-]*$` (`packages/protocol/src/plugin-config.ts`
//! line 3). That schema states no length cap, so this slice does not add
//! one: the only registrations are tests, and the release registry stays
//! empty.

use std::fmt;
use std::sync::Arc;

use crate::error::PluginError;

/// Why an id exists: hook order and the refusal both name it, and a raw
/// string would let a later match decide behaviour.
///
/// Order is the id's UTF-8 bytes (`str`'s order), not a locale.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PluginId(Arc<str>);

impl PluginId {
    /// Reject an id Paseo's schema would reject.
    ///
    /// # Errors
    ///
    /// Empty, or any character outside the pattern.
    pub fn parse(text: &str) -> Result<Self, PluginError> {
        let mut characters = text.chars();
        let Some(first) = characters.next() else {
            return Err(PluginError::EmptyId);
        };
        if !first.is_ascii_lowercase() {
            return Err(PluginError::BadId);
        }
        if !characters.all(is_id_character) {
            return Err(PluginError::BadId);
        }
        Ok(Self(Arc::from(text)))
    }

    /// The id, for a log line or a wire message.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const fn is_id_character(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
}

impl fmt::Display for PluginId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::PluginId;

    #[test]
    fn the_pattern_is_paseos() {
        assert!(PluginId::parse("a").is_ok());
        assert!(PluginId::parse("a-b2").is_ok());
        assert!(PluginId::parse("").is_err());
        assert!(PluginId::parse("A").is_err());
        assert!(PluginId::parse("1a").is_err());
        assert!(PluginId::parse("-a").is_err());
        assert!(PluginId::parse("a_b").is_err());
    }

    #[test]
    fn order_is_utf8_bytes() {
        let mut ids = vec![
            PluginId::parse("m").unwrap(),
            PluginId::parse("a").unwrap(),
            PluginId::parse("m-2").unwrap(),
        ];
        ids.sort();
        let shown: Vec<&str> = ids.iter().map(PluginId::as_str).collect();
        assert_eq!(shown, ["a", "m", "m-2"]);
    }
}
