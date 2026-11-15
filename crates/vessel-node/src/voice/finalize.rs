//! A sealed utterance gets one final-STT deadline, including worker queue delay.
//!
//! The generation's parent cancellation remains independent: completing this
//! job cancels only its child HTTP work. A timeout/cancel cannot yield text for
//! an agent prompt, and dropping a polled job also closes that child token.

use std::future::Future;
use std::time::Duration;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use vessel_proto::voice::VoiceTranscript;

use super::{
    input_buffer::Utterance,
    stt_endpoint::{Endpoint, SttError},
};

const FINAL_TRANSCRIPT_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) trait Transcriber: Send {
    fn recognize(
        &mut self,
        utterance: &Utterance,
        cancel: &CancellationToken,
    ) -> impl Future<Output = Result<VoiceTranscript, SttError>> + Send;
}
impl Transcriber for Endpoint {
    fn recognize(
        &mut self,
        utterance: &Utterance,
        cancel: &CancellationToken,
    ) -> impl Future<Output = Result<VoiceTranscript, SttError>> + Send {
        Endpoint::transcribe(self, utterance, cancel)
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum Failure {
    #[error("voice final transcription was cancelled")]
    Cancelled,
    #[error("voice final transcription exceeded its deadline")]
    TimedOut,
    #[error("voice final transcription clock is unavailable")]
    Clock,
    #[error("voice final transcription failed")]
    Transcription(#[source] SttError),
}

#[derive(Debug)]
pub(super) struct Finalization {
    utterance: Utterance,
    cancel: CancellationToken,
    deadline: Instant,
}
impl Finalization {
    /// Construct at the ending-silence edge, before queueing worker execution.
    pub(super) fn new(
        utterance: Utterance,
        generation: &CancellationToken,
    ) -> Result<Self, Failure> {
        Ok(Self {
            utterance,
            cancel: generation.child_token(),
            deadline: Instant::now()
                .checked_add(FINAL_TRANSCRIPT_TIMEOUT)
                .ok_or(Failure::Clock)?,
        })
    }
    pub(super) async fn run(
        self,
        transcriber: &mut impl Transcriber,
    ) -> Result<VoiceTranscript, Failure> {
        let Self {
            utterance,
            cancel,
            deadline,
        } = self;
        let _cancel_on_exit = cancel.clone().drop_guard();
        if cancel.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(Failure::TimedOut);
        }
        tokio::select! {
            biased;
            () = cancel.cancelled() => Err(Failure::Cancelled),
            () = tokio::time::sleep_until(deadline) => Err(Failure::TimedOut),
            result = transcriber.recognize(&utterance, &cancel) => result.map_err(Failure::Transcription),
        }
    }
}

#[cfg(test)]
mod tests;
