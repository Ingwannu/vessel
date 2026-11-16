//! Startup continuation policy. Internal checkpoints do not enter the timeline.
use crate::automation::PromptText;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unassigned until the CLI and native/web host-settings surfaces can show,
/// change and acknowledge the startup policy, and every client/dispatch hop
/// uses the shared `required_protocol` admission boundary (ADR 0098).
/// Only integration assigns the feature after those consumers are present.
pub const RESTART_RECOVERY_PROTOCOL: Option<crate::ProtocolVersion> = None;

/// Node-level startup continuation policy (ADR 0098).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RestartRecovery {
    /// Whether interrupted, eligible turns are continued at startup.
    pub enabled: bool,
    /// Exact text sent after restoring the provider's conversation.
    pub prompt: PromptText,
}
impl Default for RestartRecovery {
    fn default() -> Self {
        Self {
            enabled: false,
            prompt: PromptText::resume_default(),
        }
    }
}
impl RestartRecovery {
    /// Whether omitting this field preserves the reference defaults.
    #[must_use]
    pub fn is_default(&self) -> bool {
        !self.enabled && self.prompt.as_str() == PromptText::RESUME_DEFAULT
    }
}
impl fmt::Debug for RestartRecovery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RestartRecovery")
            .field("enabled", &self.enabled)
            .field("prompt", &"[redacted]")
            .finish()
    }
}

/// A partial startup-policy change; omission preserves either saved field.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetRestartRecovery {
    /// Enable or disable without replacing the configured prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Replace the prompt, also while recovery is disabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<PromptText>,
}
impl SetRestartRecovery {
    /// Overlay a saved partial change, preserving omitted values.
    #[must_use]
    pub fn over(&self, saved: &Self) -> Self {
        Self {
            enabled: self.enabled.or(saved.enabled),
            prompt: self.prompt.as_ref().or(saved.prompt.as_ref()).cloned(),
        }
    }
    /// The complete policy, with the reference defaults for unset fields.
    #[must_use]
    pub fn effective(&self) -> RestartRecovery {
        RestartRecovery {
            enabled: self.enabled.unwrap_or(false),
            prompt: self
                .prompt
                .clone()
                .unwrap_or_else(PromptText::resume_default),
        }
    }
}
impl fmt::Debug for SetRestartRecovery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetRestartRecovery")
            .field("enabled", &self.enabled)
            .field("prompt", &self.prompt.as_ref().map(|_| "[redacted]"))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_independent_patches_preserve_verbatim_prompt_and_hide_debug_text() {
        let defaults = RestartRecovery::default();
        assert!(!defaults.enabled);
        assert_eq!(defaults.prompt.as_str(), "Resume working.");
        assert!(defaults.is_default());
        let custom: SetRestartRecovery =
            serde_json::from_str("{\"prompt\":\"  Continue verbatim.\\n\"}").unwrap();
        let off = SetRestartRecovery {
            enabled: Some(false),
            prompt: None,
        }
        .over(&custom);
        let on = SetRestartRecovery {
            enabled: Some(true),
            prompt: None,
        }
        .over(&off);
        assert_eq!(on.effective().prompt.as_str(), "  Continue verbatim.\n");
        assert!(on.effective().enabled);
        assert_eq!(
            format!("{on:?}"),
            "SetRestartRecovery { enabled: Some(true), prompt: Some(\"[redacted]\") }"
        );
        assert_eq!(
            format!("{:?}", on.effective()),
            "RestartRecovery { enabled: true, prompt: \"[redacted]\" }"
        );
        assert!(serde_json::from_str::<SetRestartRecovery>("{\"prompt\":\" \\n\"}").is_err());
        let limit = crate::automation::AutomationText::PROMPT_MAX;
        let sample = "한";
        let boundary = sample.repeat(limit.checked_div(sample.len()).unwrap())
            + &"x".repeat(limit.checked_rem(sample.len()).unwrap());
        let prompt: PromptText = boundary.parse().unwrap();
        assert_eq!(prompt.as_str().len(), limit);
        assert!((prompt.to_string() + "x").parse::<PromptText>().is_err());
    }
}

mod checkpoint;
pub use checkpoint::{InvalidRestartGeneration, RestartCheckpoint, RestartGeneration};

#[cfg(test)]
mod admission_tests;
