//! Transient voice policy and restoration, never a user message or durable spec.
//!
//! Lifecycle owns reload orchestration and the provider owner owns transport.
//! This module only prepares bounded text and records which exact epoch owes
//! restoration; a late reload reply cannot change another conversation.

use std::fmt;

use vessel_proto::automation::{AutomationText, InvalidAutomationText, PromptText};
use vessel_proto::voice::VoiceEpoch;
use vessel_proto::workspace::trim_project_text;

const MAX_PROMPT_BYTES: usize = AutomationText::PROMPT_MAX;
const BLOCK_START: &str = "<vessel_voice_mode>";
const BLOCK_END: &str = "</vessel_voice_mode>";
const ENABLED: &str = "Voice conversation is enabled. Deliver each user-facing answer through the speak tool because the conversation may be heard without reading the screen. Before any other tool, speak a brief acknowledgement and the intended action. Give spoken progress while work takes time. Use short natural sentences. Treat the user's words as an imperfect speech transcript: proceed when the intent is clear, and ask a spoken clarifying question when the wording is ambiguous, incomplete or potentially misheard.";
const DISABLED: &str = "Voice conversation is disabled. Disregard voice-only communication instructions from earlier turns.";

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Prompt(PromptText);

impl Prompt {
    pub(super) fn new(text: &str) -> Result<Self, PromptError> {
        if text.len() > MAX_PROMPT_BYTES {
            return Err(PromptError::TooLong);
        }
        text.parse().map(Self).map_err(PromptError::Invalid)
    }

    pub(super) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for Prompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Prompt")
            .field("bytes", &self.0.as_str().len())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum PromptError {
    #[error("transient voice prompt exceeds its byte limit")]
    TooLong,
    #[error("transient voice prompt is invalid")]
    Invalid(#[source] InvalidAutomationText),
    #[error("voice prompt ownership is already active or uncertain")]
    Busy,
    #[error("voice prompt transition is out of order")]
    Order,
    #[error("voice prompt block boundary is invalid")]
    Boundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReloadOutcome {
    Applied,
    Refused,
    Uncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Applied {
    Current,
    Stale,
}

#[derive(Debug)]
enum Phase {
    Inactive,
    Enabling { epoch: VoiceEpoch, restore: Prompt },
    Active { epoch: VoiceEpoch, restore: Prompt },
    Restoring { epoch: VoiceEpoch, restore: Prompt },
    RestoreNeeded { epoch: VoiceEpoch, restore: Prompt },
}

/// The field is private so an uncertain restore cannot be mistaken for inactive.
#[derive(Debug)]
pub(super) struct Policy(Phase);

impl Default for Policy {
    fn default() -> Self {
        Self(Phase::Inactive)
    }
}

impl Policy {
    pub(super) fn enable(
        &mut self,
        epoch: VoiceEpoch,
        existing: Option<&Prompt>,
    ) -> Result<Prompt, PromptError> {
        if !matches!(self.0, Phase::Inactive) {
            return Err(PromptError::Busy);
        }
        // Prepare both bounded values before recording a reload or changing ownership.
        let base = strip(existing.map_or("", Prompt::as_str))?;
        let enabled = overlay(&base, ENABLED)?;
        let restore = overlay(&base, DISABLED)?;
        self.0 = Phase::Enabling { epoch, restore };
        Ok(enabled)
    }

    pub(super) fn enabled(
        &mut self,
        epoch: VoiceEpoch,
        outcome: ReloadOutcome,
    ) -> Result<Applied, PromptError> {
        let Phase::Enabling {
            epoch: current,
            restore,
        } = &self.0
        else {
            return if self.epoch() == Some(epoch) {
                Err(PromptError::Order)
            } else {
                Ok(Applied::Stale)
            };
        };
        if *current != epoch {
            return Ok(Applied::Stale);
        }
        self.0 = match outcome {
            ReloadOutcome::Applied => Phase::Active {
                epoch,
                restore: restore.clone(),
            },
            ReloadOutcome::Refused => Phase::Inactive,
            ReloadOutcome::Uncertain => Phase::RestoreNeeded {
                epoch,
                restore: restore.clone(),
            },
        };
        Ok(Applied::Current)
    }

    pub(super) fn restore(&mut self, epoch: VoiceEpoch) -> Result<Prompt, PromptError> {
        let (current, restore) = match &self.0 {
            Phase::Active { epoch, restore } | Phase::RestoreNeeded { epoch, restore } => {
                (*epoch, restore.clone())
            }
            Phase::Inactive | Phase::Enabling { .. } | Phase::Restoring { .. } => {
                return Err(PromptError::Order);
            }
        };
        if current != epoch {
            return Err(PromptError::Order);
        }
        self.0 = Phase::Restoring {
            epoch,
            restore: restore.clone(),
        };
        Ok(restore)
    }

    pub(super) fn restored(
        &mut self,
        epoch: VoiceEpoch,
        outcome: ReloadOutcome,
    ) -> Result<Applied, PromptError> {
        let Phase::Restoring {
            epoch: current,
            restore,
        } = &self.0
        else {
            return if self.epoch() == Some(epoch) {
                Err(PromptError::Order)
            } else {
                Ok(Applied::Stale)
            };
        };
        if *current != epoch {
            return Ok(Applied::Stale);
        }
        self.0 = match outcome {
            ReloadOutcome::Applied => Phase::Inactive,
            ReloadOutcome::Refused | ReloadOutcome::Uncertain => Phase::RestoreNeeded {
                epoch,
                restore: restore.clone(),
            },
        };
        Ok(Applied::Current)
    }

    fn epoch(&self) -> Option<VoiceEpoch> {
        match self.0 {
            Phase::Inactive => None,
            Phase::Enabling { epoch, .. }
            | Phase::Active { epoch, .. }
            | Phase::Restoring { epoch, .. }
            | Phase::RestoreNeeded { epoch, .. } => Some(epoch),
        }
    }
}

fn overlay(base: &str, instruction: &str) -> Result<Prompt, PromptError> {
    let mut text = String::new();
    if !base.is_empty() {
        text.push_str(base);
        text.push_str("\n\n");
    }
    text.push_str(BLOCK_START);
    text.push('\n');
    text.push_str(instruction);
    text.push('\n');
    text.push_str(BLOCK_END);
    Prompt::new(&text)
}

fn strip(mut input: &str) -> Result<String, PromptError> {
    let mut output = String::new();
    while let Some(start) = input.find(BLOCK_START) {
        let after = start
            .checked_add(BLOCK_START.len())
            .ok_or(PromptError::Boundary)?;
        let tail = input.get(after..).ok_or(PromptError::Boundary)?;
        let Some(end) = tail.find(BLOCK_END) else {
            break;
        };
        output.push_str(input.get(..start).ok_or(PromptError::Boundary)?);
        let next = end
            .checked_add(BLOCK_END.len())
            .ok_or(PromptError::Boundary)?;
        input = tail.get(next..).ok_or(PromptError::Boundary)?;
    }
    output.push_str(input);
    Ok(trim_project_text(&output).to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_complete_blocks_and_restores_only_the_selected_epochs_base() {
        let epoch = VoiceEpoch::random();
        let other = VoiceEpoch::random();
        let base = Prompt::new(&format!(
            "\u{feff}base {BLOCK_START}old{BLOCK_END} retained"
        ))
        .unwrap();
        let mut policy = Policy::default();
        let enabled = policy.enable(epoch, Some(&base)).unwrap();
        assert_eq!(enabled.as_str().matches(BLOCK_START).count(), 1);
        assert!(enabled.as_str().starts_with("base  retained"));
        assert_eq!(
            policy.enabled(other, ReloadOutcome::Applied),
            Ok(Applied::Stale)
        );
        assert_eq!(policy.enable(other, None), Err(PromptError::Busy));
        assert_eq!(
            policy.enabled(epoch, ReloadOutcome::Applied),
            Ok(Applied::Current)
        );
        let restore = policy.restore(epoch).unwrap();
        assert!(restore.as_str().contains(DISABLED));
        assert!(!restore.as_str().contains(ENABLED));
        assert_eq!(
            policy.restored(other, ReloadOutcome::Applied),
            Ok(Applied::Stale)
        );
        assert_eq!(
            policy.restored(epoch, ReloadOutcome::Uncertain),
            Ok(Applied::Current)
        );
        assert_eq!(policy.enable(other, None), Err(PromptError::Busy));
        assert_eq!(policy.restore(epoch).unwrap(), restore);
        assert_eq!(
            policy.restored(epoch, ReloadOutcome::Applied),
            Ok(Applied::Current)
        );
        assert!(policy.enable(other, None).is_ok());
    }

    #[test]
    fn bounds_and_redaction_precede_reload_and_refusal_does_not_leave_active_policy() {
        let epoch = VoiceEpoch::random();
        let mut policy = Policy::default();
        let full = Prompt::new(&"x".repeat(MAX_PROMPT_BYTES)).unwrap();
        assert_eq!(policy.enable(epoch, Some(&full)), Err(PromptError::TooLong));
        let private = Prompt::new("private-system-policy").unwrap();
        assert!(!format!("{private:?}").contains("private-system-policy"));
        assert!(policy.enable(epoch, Some(&private)).is_ok());
        assert_eq!(
            policy.enabled(epoch, ReloadOutcome::Refused),
            Ok(Applied::Current)
        );
        assert!(policy.enable(VoiceEpoch::random(), None).is_ok());
    }

    #[test]
    fn incomplete_reference_style_marker_is_preserved_and_uncertain_enable_owes_restore() {
        assert_eq!(
            strip(&format!("base {BLOCK_START}unclosed")).unwrap(),
            format!("base {BLOCK_START}unclosed")
        );
        let mut policy = Policy::default();
        let epoch = VoiceEpoch::random();
        assert!(policy.enable(epoch, None).is_ok());
        assert_eq!(
            policy.enabled(epoch, ReloadOutcome::Uncertain),
            Ok(Applied::Current)
        );
        assert!(policy.restore(epoch).unwrap().as_str().contains(DISABLED));
        assert_eq!(
            policy.restored(epoch, ReloadOutcome::Refused),
            Ok(Applied::Current)
        );
        assert_eq!(
            policy.enable(VoiceEpoch::random(), None),
            Err(PromptError::Busy)
        );
    }
}
