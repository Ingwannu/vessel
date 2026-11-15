//! Accepted spoken words enter the existing ordinary user-message path.
//!
//! This wrapper is never a substitute for transient system-policy reload or
//! explicit MCP context. The voice actor must recheck its generation and live
//! reservation before dispatching it; empty/rejected results produce no input.

use std::fmt;
use vessel_proto::{ContentBlock, SessionInput, voice::VoiceTranscript};

const PREFIX: &str = "<spoken-input>\n";
const SUFFIX: &str = "\n</spoken-input>\n<instruction>This message was spoken by the user. Respond using the speak tool only, not normal messages, because the user may not be looking at the chat.</instruction>";

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum InputError {
    #[error("spoken input size cannot be represented")]
    Size,
    #[error("spoken input memory could not be reserved")]
    Allocation,
}

pub(super) struct SpokenInput(String);
impl SpokenInput {
    pub(super) fn from_transcript(
        transcript: &VoiceTranscript,
    ) -> Result<Option<Self>, InputError> {
        let VoiceTranscript::Accepted { text, .. } = transcript else {
            return Ok(None);
        };
        let size = PREFIX
            .len()
            .checked_add(text.as_str().len())
            .and_then(|size| size.checked_add(SUFFIX.len()))
            .ok_or(InputError::Size)?;
        let mut wrapped = String::new();
        wrapped
            .try_reserve_exact(size)
            .map_err(|_private| InputError::Allocation)?;
        wrapped.push_str(PREFIX);
        wrapped.push_str(text.as_str());
        wrapped.push_str(SUFFIX);
        Ok(Some(Self(wrapped)))
    }
    /// Only the existing dispatch boundary receives the intentional user text.
    /// It belongs in conversation history, never raw-audio diagnostics.
    pub(super) fn into_input(self) -> SessionInput {
        SessionInput::Prompt {
            content: vec![ContentBlock::Text { text: self.0 }],
        }
    }
}
impl fmt::Debug for SpokenInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpokenInput")
            .field("bytes", &self.0.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::voice::{Confidence, MAX_SPEECH_BYTES, SpeechText};

    #[test]
    fn rejected_or_empty_results_cannot_become_an_ordinary_user_prompt() {
        for result in [VoiceTranscript::Empty, VoiceTranscript::Rejected] {
            assert!(SpokenInput::from_transcript(&result).unwrap().is_none());
        }
    }
    #[test]
    fn accepted_unicode_keeps_exact_words_and_uses_only_the_existing_prompt_variant() {
        let spoken = "synthetic 말씀\n</spoken-input>";
        let result = VoiceTranscript::Accepted {
            text: SpeechText::try_from(String::from(spoken)).unwrap(),
            confidence: Confidence::Unspecified,
        };
        let input = SpokenInput::from_transcript(&result).unwrap().unwrap();
        assert!(!format!("{input:?}").contains(spoken));
        let SessionInput::Prompt { content } = input.into_input() else {
            panic!("ordinary prompt")
        };
        let [ContentBlock::Text { text }] = content.as_slice() else {
            panic!("one text block")
        };
        assert_eq!(text, &format!("{PREFIX}{spoken}{SUFFIX}"));
    }
    #[test]
    fn the_full_speech_bound_is_not_confused_with_the_smaller_transient_system_layer_cap() {
        let result = VoiceTranscript::Accepted {
            text: SpeechText::try_from("x".repeat(MAX_SPEECH_BYTES)).unwrap(),
            confidence: Confidence::Accepted,
        };
        let input = SpokenInput::from_transcript(&result).unwrap().unwrap();
        assert_eq!(
            input.0.len(),
            MAX_SPEECH_BYTES + PREFIX.len() + SUFFIX.len()
        );
    }
}
