//! Strict configuration JSON decoding at the CLI/file boundary.
//! Temporary trees detect fields tolerated by downgrade-compatible wire/file
//! types. Neither a tree nor serde's caller-chosen diagnostic text escapes.

mod json;

use std::fmt;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{AgentDriver, ParkAfter, SetAgentDrivers, SetNodeSettings};

/// A JSON refusal contains only fixed categories and safe location metadata.
#[derive(Debug, thiserror::Error)]
pub enum InvalidSettingInput {
    /// Refuse allocation/decoding beyond the shared settings input budget.
    #[error("setting JSON exceeds {} bytes", SetNodeSettings::MAX_JSON_BYTES)]
    TooLong,
    /// Malformed JSON or a value of the wrong type, with no input in its cause.
    #[error("setting value is not valid JSON for this field")]
    Decode(#[source] SettingDecodeFailure),
    /// The canonical typed value could not be represented.
    #[error("setting value cannot be encoded")]
    Encode(#[source] SettingDecodeFailure),
    /// Never silently lose extra fields or accept an unknown enum choice.
    #[error("setting value has an unknown field or choice")]
    Unknown,
}

/// Retain category/location, replacing serde messages that can quote secrets.
/// Like the RPC decoding boundary, this intentionally has no raw error source.
#[derive(Debug)]
pub struct SettingDecodeFailure {
    category: serde_json::error::Category,
    location: Option<DecodeLocation>,
}

#[derive(Debug)]
struct DecodeLocation {
    line: std::num::NonZeroUsize,
    column: usize,
}

impl From<serde_json::Error> for SettingDecodeFailure {
    fn from(error: serde_json::Error) -> Self {
        Self {
            category: error.classify(),
            location: std::num::NonZeroUsize::new(error.line()).map(|line| DecodeLocation {
                line,
                column: error.column(),
            }),
        }
    }
}

impl fmt::Display for SettingDecodeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "JSON {:?} error", self.category)?;
        if let Some(location) = &self.location {
            write!(
                formatter,
                " at line {}, column {}",
                location.line, location.column
            )?;
        }
        Ok(())
    }
}
impl std::error::Error for SettingDecodeFailure {}

impl SetNodeSettings {
    /// Bound an explicit config value or reload document before parsing.
    /// Startup's existing downgrade-tolerant file reader remains unchanged.
    pub const MAX_JSON_BYTES: usize = 1_048_576;

    /// Decode the explicit reload snapshot before any runtime application.
    ///
    /// # Errors
    /// Oversized, malformed, unknown or ignored data refuses the entire snapshot.
    pub fn from_json(bytes: &[u8]) -> Result<Self, InvalidSettingInput> {
        known(field(bytes)?)
    }
}

/// Preserve the canonical type while refusing silent loss at this edge.
pub(super) fn field<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
) -> Result<T, InvalidSettingInput> {
    if bytes.len() > SetNodeSettings::MAX_JSON_BYTES {
        return Err(InvalidSettingInput::TooLong);
    }
    let raw = json::decode(bytes).map_err(|error| InvalidSettingInput::Decode(error.into()))?;
    let typed = T::deserialize(&raw).map_err(|error| InvalidSettingInput::Decode(error.into()))?;
    let encoded =
        serde_json::to_value(&typed).map_err(|error| InvalidSettingInput::Encode(error.into()))?;
    if raw != encoded {
        return Err(InvalidSettingInput::Unknown);
    }
    Ok(typed)
}

/// Knownness is checked before contact; driver availability remains node-owned.
pub(super) fn known(change: SetNodeSettings) -> Result<SetNodeSettings, InvalidSettingInput> {
    let SetNodeSettings {
        park_after,
        drivers,
    } = change;
    if park_after == Some(ParkAfter::Unknown)
        || drivers
            .iter()
            .flat_map(SetAgentDrivers::named)
            .any(|(_, driver)| driver == AgentDriver::Unknown)
    {
        return Err(InvalidSettingInput::Unknown);
    }
    Ok(change)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(bytes: &[u8]) -> Result<SetNodeSettings, InvalidSettingInput> {
        known(field(bytes)?)
    }

    #[test]
    fn strict_input_refuses_unknown_and_ignored_data_before_any_mutation() {
        let valid = br#"{"park_after":{"kind":"never"},"drivers":{"codex":"acp"}}"#;
        let change = decode(valid).unwrap();
        assert_eq!(change.park_after, Some(ParkAfter::Never));
        assert_eq!(change.drivers.unwrap().codex, Some(AgentDriver::Acp));
        for bytes in [
            br#"{"later":{"secret":"canary"}}"#.as_slice(),
            br#"{"park_after":{"kind":"later"}}"#,
            br#"{"drivers":{"codex":"later"}}"#,
            br#"{"drivers":{"future_agent":"acp"}}"#,
            br#"{"drivers":{"codex":null}}"#,
            br#"{"park_after":null}"#,
            br#"{"park_after":{"kind":"never","extra":true}}"#,
        ] {
            assert!(matches!(decode(bytes), Err(InvalidSettingInput::Unknown)));
        }
        assert!(decode(b"{}").is_ok());
        assert!(decode(b"{} {}").is_err());
    }

    #[test]
    fn the_byte_budget_precedes_json_parsing_and_supplies_the_diagnostic() {
        let mut at_limit = vec![b' '; SetNodeSettings::MAX_JSON_BYTES];
        for (byte, initial) in at_limit.iter_mut().zip(b"{}") {
            *byte = *initial;
        }
        assert!(decode(&at_limit).is_ok());
        at_limit.push(b'!');
        let error = decode(&at_limit).unwrap_err();
        assert!(matches!(error, InvalidSettingInput::TooLong));
        assert!(
            error
                .to_string()
                .contains(&SetNodeSettings::MAX_JSON_BYTES.to_string())
        );
    }

    #[test]
    fn every_error_view_and_source_hides_untrusted_json_values() {
        let canary = "private-setting-canary";
        for bytes in [
            format!(r#"{{"drivers":"{canary}"}}"#),
            format!(r#"{{"park_after":{{"kind":"{canary}"}}}}"#),
            format!("\"{canary}"),
        ] {
            let error = decode(bytes.as_bytes()).unwrap_err();
            assert!(!format!("{error:?}").contains(canary));
            let mut source: Option<&dyn std::error::Error> = Some(&error);
            while let Some(error) = source {
                assert!(!error.to_string().contains(canary));
                source = error.source();
            }
        }
    }
}

#[cfg(test)]
mod duplicate_tests {
    use super::super::NodeSettingKey;
    use super::*;

    fn safe_refusal(error: &InvalidSettingInput, canary: &str) {
        assert!(matches!(error, InvalidSettingInput::Decode(_)));
        let mut source: Option<&dyn std::error::Error> = Some(error);
        while let Some(error) = source {
            assert!(!error.to_string().contains(canary));
            assert!(!format!("{error:?}").contains(canary));
            source = error.source();
        }
    }

    #[test]
    fn original_document_duplicates_are_refused_before_any_member_is_lost() {
        let canary = "private-duplicate-setting-canary";
        for input in [
            format!(r#"{{"park_after":"{canary}","park_after":{{"kind":"never"}}}}"#),
            format!(r#"{{"drivers":{{"codex":"{canary}","codex":"acp"}}}}"#),
            r#"{"drivers":{"codex":null,"codex":"acp"}}"#.to_owned(),
            format!(r#"{{"park_after":{{"kind":"{canary}","kind":"never"}}}}"#),
            // Compare decoded keys, including escaped aliases, at every depth.
            r#"{"drivers":{"co\u0064ex":null,"codex":"acp"}}"#.to_owned(),
            format!(r#"{{"future":[{{"{canary}":null,"{canary}":true}}]}}"#),
        ] {
            let error = SetNodeSettings::from_json(input.as_bytes()).unwrap_err();
            safe_refusal(&error, canary);
        }
    }

    #[test]
    fn whole_field_duplicates_cannot_hide_a_prior_null_or_invalid_choice() {
        let canary = "private-duplicate-field-canary";
        for input in [
            format!(r#"{{"codex":"{canary}","codex":"acp"}}"#),
            r#"{"codex":null,"codex":"acp"}"#.to_owned(),
        ] {
            let error = NodeSettingKey::Drivers.change(&input).unwrap_err();
            safe_refusal(&error, canary);
        }
        let error = NodeSettingKey::ParkAfter
            .change(&format!(r#"{{"kind":"{canary}","kind":"never"}}"#))
            .unwrap_err();
        safe_refusal(&error, canary);
    }
}
