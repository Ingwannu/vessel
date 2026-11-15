//! The fixed transcription form, assembled in memory without a temporary WAV.
//!
//! Reqwest's multipart feature is not enabled in the workspace. This owns only
//! the four fixed text fields and a single PCM WAV, not a general form encoder.
//! Parsed metadata cannot inject a field header; generated delimiters are checked
//! against every variable part before the body is built.

use super::audio::Segment;
use bytes::Bytes;
use reqwest::header::HeaderValue;
use vessel_proto::dictation::{EndpointModel, Language};

/// What Paseo tells the provider; never supplied by the caller or read from env.
const PROMPT: &str = "Transcribe only what the speaker says. Do not add words. Preserve punctuation and casing. If the audio is silence or non-speech noise, return an empty transcript.";
const MAX_DELIMITER_ATTEMPTS: usize = 4;
const DELIMITER_BYTES: usize = size_of::<u128>();
const DELIMITER_DIGITS: usize = DELIMITER_BYTES * 2;

/// A fixed form's encoding failures retain their causes, never their payload.
#[derive(Debug, thiserror::Error)]
pub(super) enum FormError {
    #[error("a dictation multipart delimiter could not be generated")]
    Entropy(#[source] getrandom::Error),
    #[error("a dictation multipart delimiter collided after {max} attempts", max = MAX_DELIMITER_ATTEMPTS)]
    Delimiter,
    #[error("the dictation multipart content type was refused")]
    Header(#[source] reqwest::header::InvalidHeaderValue),
    #[error("the dictation WAV could not be encoded")]
    Audio(#[source] super::audio::AudioError),
}

/// Borrow validated metadata without replacing it with boundary strings.
pub(super) struct Fields<'a> {
    /// Select the endpoint model only at the multipart edge.
    pub(super) model: &'a EndpointModel,
    /// Encode the configured language at the same edge.
    pub(super) language: &'a Language,
}

/// Only the HTTP request builder can consume the completed form.
pub(super) struct Multipart {
    /// Contains audio; deliberately has no Debug implementation.
    bytes: Bytes,
    /// Kept together with its matching generated delimiter.
    content_type: HeaderValue,
}

impl Multipart {
    /// Consume a matching header/body pair only at the HTTP request edge.
    #[must_use]
    pub(super) fn into_parts(self) -> (HeaderValue, Bytes) {
        (self.content_type, self.bytes)
    }

    /// Validate delimiter collisions before retaining a second audio body.
    pub(super) fn new(segment: &Segment, fields: &Fields<'_>) -> Result<Self, FormError> {
        let boundary = Boundary::new(segment, fields)?;
        let mut body = Vec::new();
        for (name, value) in [
            ("model", fields.model.as_str()),
            ("language", fields.language.as_str()),
            ("response_format", "json"),
            ("prompt", PROMPT),
        ] {
            field(&mut body, &boundary, name, value);
        }
        body.extend_from_slice(format!("--{}\r\n", boundary.0).as_bytes());
        body.extend_from_slice(
            b"content-disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\n",
        );
        body.extend_from_slice(b"content-type: audio/wav\r\n\r\n");
        segment.append_wav(&mut body).map_err(FormError::Audio)?;
        body.extend_from_slice(format!("\r\n--{}--\r\n", boundary.0).as_bytes());
        let content_type =
            HeaderValue::from_str(&format!("multipart/form-data; boundary={}", boundary.0))
                .map_err(FormError::Header)?;
        Ok(Self {
            bytes: body.into(),
            content_type,
        })
    }
}

struct Boundary(String);

impl Boundary {
    fn new(segment: &Segment, fields: &Fields<'_>) -> Result<Self, FormError> {
        Self::with_entropy(segment, fields, || {
            let mut bytes = [0_u8; DELIMITER_BYTES];
            getrandom::fill(&mut bytes)?;
            Ok(bytes)
        })
    }

    fn with_entropy(
        segment: &Segment,
        fields: &Fields<'_>,
        mut entropy: impl FnMut() -> Result<[u8; DELIMITER_BYTES], getrandom::Error>,
    ) -> Result<Self, FormError> {
        for _ in 0..MAX_DELIMITER_ATTEMPTS {
            // A generic entropy edge lets tests force collision/failure without
            // a second product backend or an unbounded rejection loop.
            let random = entropy().map_err(FormError::Entropy)?;
            let candidate = Self(format!(
                "{:0width$x}",
                u128::from_be_bytes(random),
                width = DELIMITER_DIGITS
            ));
            if !candidate.collides(segment, fields) {
                return Ok(candidate);
            }
        }
        Err(FormError::Delimiter)
    }

    fn collides(&self, segment: &Segment, fields: &Fields<'_>) -> bool {
        [
            segment.as_slice(),
            fields.model.as_str().as_bytes(),
            fields.language.as_str().as_bytes(),
            PROMPT.as_bytes(),
        ]
        .into_iter()
        .any(|part| {
            part.windows(self.0.len())
                .any(|window| window == self.0.as_bytes())
        })
    }
}

fn field(body: &mut Vec<u8>, boundary: &Boundary, name: &str, value: &str) {
    body.extend_from_slice(
        format!(
            "--{}\r\ncontent-disposition: form-data; name=\"{name}\"\r\n\r\n",
            boundary.0
        )
        .as_bytes(),
    );
    body.extend_from_slice(value.as_bytes());
    body.extend_from_slice(b"\r\n");
}

#[cfg(test)]
#[path = "multipart_tests.rs"]
mod tests;
