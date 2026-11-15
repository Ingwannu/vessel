//! Voice multipart policy over the reviewed shared PCM/resampler/WAV codec.
//!
//! Unlike dictation this sends no fixed prompt. Only models advertising the
//! voice logprob policy request include[]=logprobs. Payload Debug stays redacted.

use reqwest::header::HeaderValue;
use std::fmt;
use vessel_proto::dictation::{EndpointModel, Language};
use vessel_proto::voice::CAPTURE_RATE_HZ;

use super::input_buffer::{MAX_UTTERANCE_BYTES, Utterance};
use super::transcript::Scores;
use crate::speech_audio::{PROVIDER_RATE, Pcm16, Rate, Resampler, Wav};

const MAX_PROVIDER_BYTES: usize = 2_880_000;
const FORM_OVERHEAD: usize = 2_048;
const MAX_FORM_BYTES: usize = MAX_PROVIDER_BYTES + FORM_OVERHEAD;

#[derive(Debug, thiserror::Error)]
pub(super) enum FormError {
    #[error("voice utterance is empty or exceeds its byte budget")]
    Input,
    #[error("voice provider request exceeds its byte budget")]
    Capacity,
    #[error("voice provider request could not reserve memory")]
    Allocation(#[source] std::collections::TryReserveError),
    #[error("voice audio could not be encoded")]
    Codec(#[from] crate::speech_audio::CodecError),
    #[error("voice provider request metadata is invalid")]
    Metadata,
}

pub(super) struct Form {
    body: Vec<u8>,
    content_type: HeaderValue,
}
impl fmt::Debug for Form {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceForm")
            .field("bytes", &self.body.len())
            .finish()
    }
}

impl Form {
    pub(super) fn new(
        utterance: &Utterance,
        model: &EndpointModel,
        language: &Language,
    ) -> Result<Self, FormError> {
        let input = utterance.pcm16();
        if input.is_empty() || input.len() > MAX_UTTERANCE_BYTES {
            return Err(FormError::Input);
        }
        let rate = Rate::new(CAPTURE_RATE_HZ).map_err(|_private| FormError::Metadata)?;
        let mut converter = Resampler::new(rate);
        let pcm = converter.push(Pcm16::new(input)?)?;
        if pcm.as_slice().len() > MAX_PROVIDER_BYTES {
            return Err(FormError::Capacity);
        }
        let wav = Wav::new(PROVIDER_RATE, pcm.as_pcm())?;
        let boundary = format!("vessel-voice-{}", uuid::Uuid::new_v4().simple());
        let content_type =
            HeaderValue::from_str(&format!("multipart/form-data; boundary={boundary}"))
                .map_err(|_private| FormError::Metadata)?;
        let mut body = Vec::new();
        body.try_reserve(pcm.as_slice().len().saturating_add(FORM_OVERHEAD))
            .map_err(FormError::Allocation)?;
        for (name, value) in [
            ("model", model.as_str()),
            ("language", language.as_str()),
            ("response_format", "json"),
        ] {
            field(&mut body, &boundary, name, value)?;
        }
        if Scores::for_model(model) == Scores::Requested {
            field(&mut body, &boundary, "include[]", "logprobs")?;
        }
        append(&mut body, format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"utterance.wav\"\r\nContent-Type: audio/wav\r\n\r\n").as_bytes())?;
        wav.append_to(&mut body)?;
        append(&mut body, format!("\r\n--{boundary}--\r\n").as_bytes())?;
        Ok(Self { body, content_type })
    }
    pub(super) fn into_parts(self) -> (HeaderValue, Vec<u8>) {
        (self.content_type, self.body)
    }
}

fn field(body: &mut Vec<u8>, boundary: &str, name: &str, value: &str) -> Result<(), FormError> {
    append(
        body,
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
        )
        .as_bytes(),
    )
}
fn append(body: &mut Vec<u8>, bytes: &[u8]) -> Result<(), FormError> {
    if body
        .len()
        .checked_add(bytes.len())
        .is_none_or(|length| length > MAX_FORM_BYTES)
    {
        return Err(FormError::Capacity);
    }
    body.try_reserve(bytes.len())
        .map_err(FormError::Allocation)?;
    body.extend_from_slice(bytes);
    Ok(())
}

#[cfg(test)]
mod tests;
