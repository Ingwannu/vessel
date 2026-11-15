//! Typed stream failures retained until their one RPC conversion.

use super::{
    audio::AudioError, budget::BudgetError, endpoint::ProviderError, input::InputError,
    jobs::JobError, provider_json::DecodeError, text::TextError,
};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use vessel_proto::dictation::DictationReason;

pub(super) type StreamResult<T> = Result<T, Arc<StreamError>>;

#[derive(thiserror::Error)]
pub(super) enum StreamError {
    #[error("the dictation input was refused")]
    Input(#[from] InputError),
    #[error("the dictation audio could not be processed")]
    Audio(#[from] AudioError),
    #[error("the dictation transcript could not be assembled")]
    Text(#[from] TextError),
    #[error("the dictation finish budget could not be computed")]
    Budget(#[from] BudgetError),
    #[error("the dictation provider could not complete the segment")]
    Provider(#[from] JobError),
    #[error("the dictation stream no longer exists")]
    Gone,
    #[error("the dictation clock could not represent its deadline")]
    Clock,
    #[error("the dictation provider job bound was exceeded")]
    Capacity,
    #[error("the dictation update buffer refused a partial")]
    Partial(#[from] super::updates::PartialRefusal),
    #[error("the dictation terminal update was refused")]
    Terminal(#[from] super::updates::EndRefusal<Arc<StreamError>>),
    #[error("the dictation transcript failed validation")]
    Transcript(#[source] vessel_proto::dictation::DictationDecodeError),
    #[error("the dictation update wait was refused")]
    Read(#[from] super::updates::ReadRefusal),
    #[error("dictation finish reached its deadline")]
    TimedOut,
    #[error("the dictation stream was replaced")]
    Replaced,
    #[error("the dictation stream was cancelled")]
    Cancelled,
    #[error("the dictation stream closed before accepting work")]
    Send(#[source] mpsc::error::SendError<()>),
    #[error("the dictation stream closed without its reply")]
    Reply(#[source] oneshot::error::RecvError),
}
impl std::fmt::Debug for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl StreamError {
    pub(super) fn reason(&self) -> Option<DictationReason> {
        Some(match self {
            Self::Input(error) => match error {
                InputError::BadAudio => DictationReason::BadAudio,
                InputError::TooLong => DictationReason::TooLong,
                InputError::ReorderOverflow => DictationReason::ReorderOverflow,
                InputError::NoAudio => DictationReason::NoAudio,
                InputError::Count(_) => DictationReason::Upstream,
            },
            Self::Text(error) => match error {
                TextError::TooLong | TextError::Transcript(_) => DictationReason::TranscriptTooLong,
                TextError::Commit | TextError::Boundary => DictationReason::Upstream,
            },
            Self::Provider(error) => provider_reason(error),
            Self::Transcript(_) => DictationReason::TranscriptTooLong,
            Self::Terminal(_)
            | Self::Audio(_)
            | Self::Budget(_)
            | Self::Clock
            | Self::Capacity
            | Self::Partial(_) => DictationReason::Upstream,
            Self::TimedOut => DictationReason::TimedOut,
            Self::Replaced => DictationReason::Replaced,
            Self::Cancelled => DictationReason::Cancelled,
            Self::Gone | Self::Send(_) | Self::Reply(_) | Self::Read(_) => return None,
        })
    }
}
fn provider_reason(error: &JobError) -> DictationReason {
    match error {
        JobError::Endpoint(error) => match error {
            ProviderError::Decode(error) => match error {
                DecodeError::Transcript(_) => DictationReason::TranscriptTooLong,
                DecodeError::Json(_)
                | DecodeError::MissingText
                | DecodeError::Status(_)
                | DecodeError::StatusBody { .. }
                | DecodeError::TooSmall
                | DecodeError::CredentialEcho => DictationReason::Upstream,
            },
            ProviderError::Http(_)
            | ProviderError::Roots(_)
            | ProviderError::KeyAbsent
            | ProviderError::Address(_)
            | ProviderError::Header(_)
            | ProviderError::Form(_)
            | ProviderError::ResponseTooLarge
            | ProviderError::Count(_) => DictationReason::Upstream,
        },
        JobError::Join(_) => DictationReason::Upstream,
        #[cfg(test)]
        JobError::Send(_) | JobError::Reply(_) => DictationReason::Upstream,
    }
}
