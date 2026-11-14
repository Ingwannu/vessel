//! Decoder failures contain reasons and causes, never audio or transcript text.

/// Invalid dictation input, refused before a node opens a provider.
#[derive(Debug, thiserror::Error)]
pub enum DictationDecodeError {
    /// IDs must be safe to include in structured diagnostics.
    #[error("a dictation id is 1 to 64 ASCII letters, digits, underscores or hyphens")]
    Id,
    /// PCM is the only input encoding in the streaming contract.
    #[error("dictation requires audio/pcm with one rate, bits=16 and optional channels=1")]
    Format,
    /// Preserve the numeric cause without reflecting user input.
    #[error("the dictation sample rate must be an integer")]
    RateSyntax(#[source] std::num::ParseIntError),
    /// Bound resampling work before accepting any audio.
    #[error("the dictation sample rate must be from 8000 to 48000")]
    RateRange,
    /// Bound decoded audio before a stream retains it.
    #[error("dictation audio must contain complete PCM16 samples, from 2 bytes to 256 KiB")]
    Pcm,
    /// The decoding source carries no audio bytes.
    #[error("dictation audio must be valid base64")]
    Encoding(#[source] base64::DecodeError),
    /// Refuse rather than truncate the provider's answer.
    #[error("a dictation transcript is at most 64 KiB")]
    Transcript,
    /// A peer may not expand the agreed maximum finish wait.
    #[error("a dictation finish budget is at most 300000 milliseconds")]
    Budget,
}
