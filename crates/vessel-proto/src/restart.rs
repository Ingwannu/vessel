//! Startup continuation policy. Internal checkpoints do not enter the timeline.
use crate::automation::PromptText;
use serde::{Deserialize, Serialize};
use std::fmt;

/// First protocol preserving startup policy through every decoding hop.
pub const RESTART_RECOVERY_PROTOCOL: crate::ProtocolVersion = crate::ProtocolVersion::new(20);

/// Node-level startup continuation policy (ADR 0059).
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
        !self.enabled && self.prompt.as_str() == "Resume working."
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

/// A stop/admission generation, bounded by the store's integer range.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct RestartGeneration(u64);
/// A generation that cannot be represented by the durable projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("restart generation exceeds the durable counter range")]
pub struct InvalidRestartGeneration;
impl RestartGeneration {
    /// First generation of a session.
    pub const ZERO: Self = Self(0);
    /// The durable counter value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
    /// The next generation, refusing exhaustion rather than wrapping.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        self.0
            .checked_add(1)
            .and_then(|value| Self::try_from(value).ok())
    }
}
impl TryFrom<u64> for RestartGeneration {
    type Error = InvalidRestartGeneration;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if i64::try_from(value).is_ok() {
            Ok(Self(value))
        } else {
            Err(InvalidRestartGeneration)
        }
    }
}
impl From<RestartGeneration> for u64 {
    fn from(value: RestartGeneration) -> Self {
        value.0
    }
}

/// An internal ordered persistence checkpoint, never a public timeline event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartCheckpoint {
    /// A newly accepted prompt's actual `TurnStarted` record permits recovery.
    Arm {
        /// Generation captured at this input's admission.
        generation: RestartGeneration,
        /// The exact turn, not an observed process status.
        turn: crate::EventSeq,
    },
    /// A person's stop intent must commit before its effect/acknowledgement.
    Suppress {
        /// New stop generation.
        generation: RestartGeneration,
        /// Only this turn when named; a stale turn is a no-op.
        turn: Option<crate::EventSeq>,
    },
    /// Capture the actor before shutdown signals its provider.
    Shutdown {
        /// Current stop/admission generation.
        generation: RestartGeneration,
        /// Actual active turn; None for idle or still starting.
        turn: Option<crate::EventSeq>,
        /// A parked agent is never reopened by startup recovery.
        parked: bool,
    },
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
        let boundary = "한".repeat(5461) + "x";
        let prompt: PromptText = boundary.parse().unwrap();
        assert_eq!(prompt.as_str().len(), 16384);
        assert!((prompt.to_string() + "x").parse::<PromptText>().is_err());
    }
    #[test]
    fn generations_never_wrap_or_decode_outside_the_database_range() {
        let last = RestartGeneration::try_from(u64::try_from(i64::MAX).unwrap()).unwrap();
        assert_eq!(last.next(), None);
        assert!(RestartGeneration::try_from(u64::MAX).is_err());
        assert!(serde_json::from_str::<RestartGeneration>(&u64::MAX.to_string()).is_err());
        assert_eq!(RestartGeneration::ZERO.next().unwrap().get(), 1);
    }
}
