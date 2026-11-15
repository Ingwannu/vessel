//! The default-off agent switch does not grant authority to the human control API.

use serde::{Deserialize, Deserializer, Serialize};

/// Unknown saved values stay disabled; only an explicit supported On can offer tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum BrowserTools {
    /// Fresh installs and older settings files offer no browser tools.
    #[default]
    Off,
    /// Still requires a qualified engine, Operate and the session-start admission.
    On,
    /// A later setting cannot silently turn browser access on after downgrade.
    Unknown,
}
// This preference's JSON representation is a string, never Serde's alternative
// externally-tagged unit object. In particular {"on":null} must not enable it.
impl<'de> Deserialize<'de> for BrowserTools {
    fn deserialize<D: Deserializer<'de>>(source: D) -> Result<Self, D::Error> {
        let value = String::deserialize(source)?;
        Ok(match value.as_str() {
            "off" => Self::Off,
            "on" => Self::On,
            _future => Self::Unknown,
        })
    }
}

impl BrowserTools {
    /// Keep an absent/default field compatible with older saved files and replies.
    #[must_use]
    pub fn is_off(&self) -> bool {
        !matches!(self, Self::On)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_string_choices_can_be_read_as_a_browser_preference() {
        for raw in [
            r#"{"on":null}"#,
            r#"{"off":null}"#,
            r#"{"unknown":null}"#,
            "null",
            "false",
            "[]",
            "{}",
        ] {
            assert!(serde_json::from_str::<BrowserTools>(raw).is_err());
        }
        assert_eq!(
            serde_json::from_str::<BrowserTools>(r#""on""#).unwrap(),
            BrowserTools::On
        );
        assert_eq!(
            serde_json::from_str::<BrowserTools>(r#""future""#).unwrap(),
            BrowserTools::Unknown
        );
    }
}
