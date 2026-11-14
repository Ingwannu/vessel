//! Decoder failures contain reasons and causes, never audio or transcript text.

use super::{
    KIB_BYTES, MAX_CHUNK_BYTES, MAX_FINISH_MS, MAX_ID_CHARS, MAX_RATE, MAX_TRANSCRIPT_BYTES,
    MIN_ID_CHARS, MIN_RATE, PCM_SAMPLE_BYTES,
};

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
}
