//! The host's saved supplement is distinct from a session or user prompt.
//! Its exact text can only be read through the Admin settings operation.

use std::{fmt, str::FromStr, sync::Arc};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use crate::{ProtocolVersion, automation::{AutomationText, InvalidAutomationText}};

/// Saved host prompts and the private read share one receiving-peer threshold.
pub const APPEND_SYSTEM_PROMPT_PROTOCOL: Option<ProtocolVersion> = None;

/// A bounded saved host supplement. A blank value clears the supplement.
#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct AppendSystemPrompt(Arc<str>);

impl AppendSystemPrompt {
    /// The effective value for a harness's native system instructions.
    #[must_use]
    pub fn effective(&self) -> &str {
        self.0.trim()
    }
}

impl FromStr for AppendSystemPrompt {
    type Err = InvalidAutomationText;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        AutomationText::validate_prompt_bytes(text)
            .map_err(|problem| InvalidAutomationText {
                kind: AutomationText::PromptText,
                problem,
            })?;
        Ok(Self(Arc::from(text)))
    }
}

impl fmt::Debug for AppendSystemPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AppendSystemPrompt(<redacted>)")
    }
}

impl Serialize for AppendSystemPrompt {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AppendSystemPrompt {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?.parse().map_err(D::Error::custom)
    }
}

/// A native transport proven to accept a launch supplement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AppendPromptTransport {
    /// Codex app-server `developerInstructions`, independent of the vendor base.
    CodexAppServer,
    /// A later node's transport.
    #[serde(other)]
    Unknown,
}

/// `node.append_system_prompt`: private settings text, available to Admins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetAppendSystemPrompt {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_saved_prompt_retains_text_while_launches_trim_it() {
        let raw = " \n Synthetic instruction.\n ";
        let value = raw.parse::<AppendSystemPrompt>().unwrap();
        assert_eq!(serde_json::to_value(&value).unwrap(), raw);
        assert_eq!(value.effective(), "Synthetic instruction.");
        assert_eq!(" \n".parse::<AppendSystemPrompt>().unwrap().effective(), "");
        assert!(!format!("{value:?}").contains("Synthetic"));
        assert!("internal\0text".parse::<AppendSystemPrompt>().is_err());
        assert!(" ".repeat(AutomationText::PROMPT_MAX + 1).parse::<AppendSystemPrompt>().is_err());
    }
}
