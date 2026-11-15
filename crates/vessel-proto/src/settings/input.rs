//! Strict configuration JSON decoding at the CLI/file boundary.
//! Temporary trees detect fields tolerated by downgrade-compatible wire/file
//! types. Neither a tree nor serde's caller-chosen diagnostic text escapes.

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
    line: usize,
    column: usize,
}

impl From<serde_json::Error> for SettingDecodeFailure {
    fn from(error: serde_json::Error) -> Self {
        Self {
            category: error.classify(),
            line: error.line(),
            column: error.column(),
        }
    }
}

impl fmt::Display for SettingDecodeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "JSON {:?} error at line {}, column {}",
            self.category, self.line, self.column
        )
    }
}
impl std::error::Error for SettingDecodeFailure {}

impl SetNodeSettings {
    /// Bound an explicit config value or reload document before parsing.
    /// Startup's existing downgrade-tolerant file reader remains unchanged.
    pub const MAX_JSON_BYTES: usize = 1_048_576;
}

/// Preserve the canonical type while refusing silent loss at this edge.
pub(super) fn field<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
) -> Result<T, InvalidSettingInput> {
    if bytes.len() > SetNodeSettings::MAX_JSON_BYTES {
        return Err(InvalidSettingInput::TooLong);
    }
    let raw: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| InvalidSettingInput::Decode(error.into()))?;
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
