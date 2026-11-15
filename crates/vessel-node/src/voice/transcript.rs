//! Voice-only JSON/confidence policy; dictation's fixed-prompt decoder stays separate.
//!
//! Foreign JSON dies here. Invalid/absent logprob arrays keep the reference's
//! unspecified confidence; an overflowing mean is a typed provider failure.
//! Neither rejected text nor JSON errors are retained in diagnostics.

mod response;
use response::{Response, Token, Tolerant};
use vessel_proto::dictation::EndpointModel;
use vessel_proto::voice::{Confidence, SpeechText, VoiceTranscript};
use vessel_proto::workspace::trim_project_text;

const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const LOW_CONFIDENCE_THRESHOLD: f64 = -3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scores {
    Requested,
    Unavailable,
}

impl Scores {
    pub(super) fn for_model(model: &EndpointModel) -> Self {
        match model.as_str() {
            "gpt-4o-transcribe" | "gpt-4o-mini-transcribe" => Self::Requested,
            _ => Self::Unavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum TranscriptError {
    #[error("transcription response exceeds its byte limit")]
    TooLarge,
    #[error("transcription response is invalid")]
    Invalid,
    #[error("transcription text exceeds its byte limit")]
    Text,
}

pub(super) fn decode(body: &[u8], scores: Scores) -> Result<VoiceTranscript, TranscriptError> {
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(TranscriptError::TooLarge);
    }
    let value: Response<'_> =
        serde_json::from_slice(body).map_err(|_private| TranscriptError::Invalid)?;
    let text = value.text.as_ref();
    if trim_project_text(text).is_empty() {
        return Ok(VoiceTranscript::Empty);
    }
    let mean = match scores {
        Scores::Requested => average(&value.logprobs)?,
        Scores::Unavailable => None,
    };
    if mean.is_some_and(|mean| mean < LOW_CONFIDENCE_THRESHOLD) {
        return Ok(VoiceTranscript::Rejected);
    }
    let text = SpeechText::try_from(text.to_owned()).map_err(|_private| TranscriptError::Text)?;
    Ok(VoiceTranscript::Accepted {
        text,
        confidence: mean.map_or(Confidence::Unspecified, |_| Confidence::Accepted),
    })
}

#[expect(
    clippy::float_arithmetic,
    reason = "provider finite logprob mean, checked after every sum and division"
)]
fn average(scores: &Tolerant<Vec<Token>>) -> Result<Option<f64>, TranscriptError> {
    let Some(tokens) = scores.known().filter(|tokens| !tokens.is_empty()) else {
        return Ok(None);
    };
    if tokens
        .iter()
        .any(|token| !token.bytes_valid || !token.logprob.is_finite())
    {
        return Ok(None);
    }
    let mut count = 0_u32;
    let mut sum = 0.0_f64;
    for token in tokens {
        sum += token.logprob;
        if !sum.is_finite() {
            return Err(TranscriptError::Invalid);
        }
        count = count.checked_add(1).ok_or(TranscriptError::TooLarge)?;
    }
    let mean = sum / f64::from(count);
    if !mean.is_finite() {
        return Err(TranscriptError::Invalid);
    }
    Ok(Some(mean))
}

#[cfg(test)]
mod tests;
