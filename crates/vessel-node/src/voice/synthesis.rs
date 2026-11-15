//! Ordered speech with two prefetched segments; no detached provider tasks.
//!
//! The voice actor supplies an authorized exact lease and owns one active
//! speech-job permit. Output opens the canonical receipt before queueing PCM,
//! checks ownership again, and reports actual Played/Failed/Interrupted only.
//! These preconditions are adapter contracts, not inferred readiness here.

use super::{
    receipts::{self, ReceiptError},
    segments,
    tts_endpoint::{Endpoint, Synthesized, TtsError},
};
use futures_util::{StreamExt as _, stream::FuturesOrdered};
use std::{collections::VecDeque, future::Future, time::Duration};
use tokio_util::sync::CancellationToken;
use vessel_proto::voice::{
    Playback, PlaybackId, PlaybackOutcome, SpeechGroupId, SpeechText, VoiceDecodeError, VoiceLease,
};

const PREFETCH_SEGMENTS: usize = 2;
const PROVIDER_TIMEOUT: Duration = Duration::from_secs(60);

pub(super) trait Synthesizer: Sync {
    fn synthesize(
        &self,
        text: &SpeechText,
        cancel: &CancellationToken,
    ) -> impl Future<Output = Result<Synthesized, TtsError>> + Send;
}
impl Synthesizer for Endpoint {
    fn synthesize(
        &self,
        text: &SpeechText,
        cancel: &CancellationToken,
    ) -> impl Future<Output = Result<Synthesized, TtsError>> + Send {
        Endpoint::synthesize(self, text, cancel)
    }
}
pub(super) trait Output: Send {
    fn play(
        &mut self,
        lease: VoiceLease,
        playback: Playback,
        cancel: &CancellationToken,
    ) -> impl Future<Output = Result<PlaybackOutcome, Failure>> + Send;
}
#[derive(Debug, thiserror::Error)]
pub(super) enum Failure {
    #[error("spoken text could not be segmented")]
    Text(#[from] VoiceDecodeError),
    #[error("speech synthesis failed")]
    Provider(#[source] TtsError),
    #[error("speech playback exceeds its resource limit")]
    Receipt(#[from] ReceiptError),
    #[error("spoken output exceeded its deadline")]
    TimedOut,
    #[error("voice playback transport is unavailable")]
    Output,
}

pub(super) async fn speak(
    text: &SpeechText,
    lease: VoiceLease,
    provider: &impl Synthesizer,
    output: &mut impl Output,
    generation: &CancellationToken,
) -> Result<PlaybackOutcome, Failure> {
    let cancel = generation.child_token();
    let _cancel_on_exit = cancel.clone().drop_guard();
    if cancel.is_cancelled() {
        return Ok(PlaybackOutcome::Interrupted);
    }
    let segments = segments::split(text)?;
    if segments.is_empty() {
        return Err(VoiceDecodeError::Speech.into());
    }
    let mut remaining = segments.iter();
    let mut jobs = FuturesOrdered::new();
    let mut ready = VecDeque::new();
    for segment in remaining.by_ref().take(PREFETCH_SEGMENTS) {
        jobs.push_back(synthesize(provider, segment, &cancel));
    }
    loop {
        if cancel.is_cancelled() {
            return Ok(PlaybackOutcome::Interrupted);
        }
        let prepared = match ready.pop_front() {
            Some(result) => Some(result),
            None => tokio::select! { biased;
                () = cancel.cancelled() => return Ok(PlaybackOutcome::Interrupted),
                result = jobs.next() => result,
            },
        };
        let Some(prepared) = prepared else {
            return Ok(PlaybackOutcome::Played);
        };
        let audio = prepared?;
        while jobs.len().saturating_add(ready.len()) < PREFETCH_SEGMENTS {
            let Some(segment) = remaining.next() else {
                break;
            };
            jobs.push_back(synthesize(provider, segment, &cancel));
        }
        let playback = Playback {
            id: PlaybackId::random(),
            group: SpeechGroupId::random(),
            format: audio.format,
            pcm: audio.pcm,
        };
        let wait = receipts::wait_duration(&playback)?;
        if cancel.is_cancelled() {
            return Ok(PlaybackOutcome::Interrupted);
        }
        let played = output.play(lease, playback, &cancel);
        let deadline = tokio::time::sleep(wait);
        tokio::pin!(played, deadline);
        let outcome = loop {
            tokio::select! { biased;
                () = cancel.cancelled() => return Ok(PlaybackOutcome::Interrupted),
                () = &mut deadline => return Err(Failure::TimedOut),
                result = &mut played => break result?,
                result = jobs.next(), if !jobs.is_empty() => {
                    if let Some(result) = result { ready.push_back(result); }
                },
            }
        };
        if outcome != PlaybackOutcome::Played {
            return Ok(outcome);
        }
    }
}

async fn synthesize(
    provider: &impl Synthesizer,
    text: &SpeechText,
    cancel: &CancellationToken,
) -> Result<Synthesized, Failure> {
    tokio::select! { biased;
        () = tokio::time::sleep(PROVIDER_TIMEOUT) => Err(Failure::TimedOut),
        result = provider.synthesize(text, cancel) => result.map_err(Failure::Provider),
    }
}

#[cfg(test)]
mod tests;
