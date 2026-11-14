//! Decoder failures contain reasons and causes, never audio or transcript text.

use super::{
    KIB_BYTES, MAX_CHUNK_BYTES, MAX_FINISH_MS, MAX_ID_CHARS, MAX_RATE, MAX_TRANSCRIPT_BYTES,
    MIN_ID_CHARS, MIN_RATE, PCM_SAMPLE_BYTES,
};

use super::settings_values::{MAX_LANGUAGE_CHARS, MAX_MODEL_CHARS, MAX_URL_BYTES, MIN_TEXT_CHARS};

/// Invalid dictation input, refused before a node opens a provider.
#[derive(Debug, thiserror::Error)]
pub enum DictationDecodeError {
    /// IDs must be safe to include in structured diagnostics.
    #[error("a dictation id is {min} to {max} ASCII letters, digits, underscores or hyphens", min = MIN_ID_CHARS, max = MAX_ID_CHARS)]
    Id,
    /// PCM is the only input encoding in the streaming contract.
    #[error("dictation requires audio/pcm with one rate, bits=16 and optional channels=1")]
    Format,
    /// Preserve the numeric cause without reflecting user input.
    #[error("the dictation sample rate must be an integer")]
    RateSyntax(#[source] std::num::ParseIntError),
    /// Bound resampling work before accepting any audio.
    #[error("the dictation sample rate must be from {min} to {max}", min = MIN_RATE, max = MAX_RATE)]
    RateRange,
    /// Bound decoded audio before a stream retains it.
    #[error("dictation audio must contain complete PCM16 samples, from {min} bytes to {max} KiB", min = PCM_SAMPLE_BYTES, max = MAX_CHUNK_BYTES.div_euclid(KIB_BYTES))]
    Pcm,
    /// The decoding source carries no audio bytes.
    #[error("dictation audio must be valid base64")]
    Encoding(#[source] base64::DecodeError),
    /// Refuse rather than truncate the provider's answer.
    #[error("a dictation transcript is at most {max} KiB", max = MAX_TRANSCRIPT_BYTES.div_euclid(KIB_BYTES))]
    Transcript,
    /// A peer may not expand the agreed maximum finish wait.
    #[error("a dictation finish budget is at most {max} milliseconds", max = MAX_FINISH_MS)]
    Budget,
    /// Language metadata is bounded before retaining a stream setting.
    #[error("a dictation language is {min} to {max} ASCII letters or hyphens", min = MIN_TEXT_CHARS, max = MAX_LANGUAGE_CHARS)]
    Language,
    /// A model cannot create arbitrary multipart fields.
    #[error("a dictation endpoint model is {min} to {max} characters without whitespace or controls", min = MIN_TEXT_CHARS, max = MAX_MODEL_CHARS)]
    EndpointModel,
    /// URL syntax causes never carry the supplied URI.
    #[error("a dictation endpoint base must be a valid URL")]
    EndpointUrl(#[source] url::ParseError),
    /// Keys may leave the machine only over TLS and never in the URI.
    #[error("a dictation endpoint base is at most {max} canonical bytes, HTTPS or loopback HTTP, without userinfo, query or fragment", max = MAX_URL_BYTES)]
    EndpointBase,
    /// Catalogue IDs remain tokens even when read from a newer node.
    #[error("a dictation local model is {min} to {max} ASCII letters, digits, dots, underscores or hyphens", min = MIN_TEXT_CHARS, max = MAX_MODEL_CHARS)]
    LocalModel,
}
