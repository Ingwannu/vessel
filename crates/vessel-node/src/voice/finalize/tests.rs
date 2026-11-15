use super::super::{http::EndpointError, input_buffer::InputBuffer};
use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::oneshot;
use vessel_proto::voice::{CapturePcm, Confidence, SpeechText};

struct Fake {
    calls: Arc<AtomicUsize>,
    entered: Option<oneshot::Sender<CancellationToken>>,
    reply: Option<Result<VoiceTranscript, SttError>>,
}
impl Transcriber for Fake {
    async fn transcribe(
        &mut self,
        _utterance: &Utterance,
        cancel: &CancellationToken,
    ) -> Result<VoiceTranscript, SttError> {
        let _previous = self.calls.fetch_add(1, Ordering::AcqRel);
        if let Some(entered) = self.entered.take() {
            entered.send(cancel.clone()).unwrap();
        }
        match self.reply.take() {
            Some(reply) => reply,
            None => std::future::pending().await,
        }
    }
}
fn utterance() -> Utterance {
    let mut input = InputBuffer::default();
    input
        .append(&CapturePcm::new(vec![0; 640]).unwrap())
        .unwrap();
    input.confirmed().unwrap();
    input.seal().unwrap()
}
fn fake() -> (Fake, oneshot::Receiver<CancellationToken>, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let (entered, observed) = oneshot::channel();
    (
        Fake {
            calls: Arc::clone(&calls),
            entered: Some(entered),
            reply: None,
        },
        observed,
        calls,
    )
}

#[tokio::test(start_paused = true)]
async fn queue_delay_counts_and_an_expired_utterance_never_opens_a_provider() {
    let generation = CancellationToken::new();
    let finalization = Finalization::new(utterance(), &generation).unwrap();
    tokio::time::advance(FINAL_TRANSCRIPT_TIMEOUT).await;
    let (mut provider, _observed, calls) = fake();
    assert!(matches!(
        finalization.run(&mut provider).await,
        Err(Failure::TimedOut)
    ));
    assert_eq!(calls.load(Ordering::Acquire), 0);
    assert!(!generation.is_cancelled());
}

#[tokio::test(start_paused = true)]
async fn a_held_provider_hits_the_reference_deadline_and_cancels_its_http_child() {
    let generation = CancellationToken::new();
    let finalization = Finalization::new(utterance(), &generation).unwrap();
    let (mut provider, observed, calls) = fake();
    let task = tokio::spawn(async move { finalization.run(&mut provider).await });
    let child = observed.await.unwrap();
    assert!(!child.is_cancelled());
    tokio::time::advance(FINAL_TRANSCRIPT_TIMEOUT).await;
    assert!(matches!(task.await.unwrap(), Err(Failure::TimedOut)));
    assert!(child.is_cancelled());
    assert_eq!(calls.load(Ordering::Acquire), 1);
    assert!(!generation.is_cancelled());
}

#[tokio::test]
async fn stop_and_dropped_workers_cancel_jobs_without_cancelling_the_parent_for_next_turn() {
    let generation = CancellationToken::new();
    let finalization = Finalization::new(utterance(), &generation).unwrap();
    let (mut provider, observed, _calls) = fake();
    let task = tokio::spawn(async move { finalization.run(&mut provider).await });
    let child = observed.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(child.is_cancelled());
    assert!(!generation.is_cancelled());
    let finalization = Finalization::new(utterance(), &generation).unwrap();
    let (mut provider, _observed, calls) = fake();
    generation.cancel();
    assert!(matches!(
        finalization.run(&mut provider).await,
        Err(Failure::Cancelled)
    ));
    assert_eq!(calls.load(Ordering::Acquire), 0);
}

#[tokio::test]
async fn empty_rejected_accepted_and_failed_results_keep_their_closed_types() {
    let generation = CancellationToken::new();
    for transcript in [
        VoiceTranscript::Empty,
        VoiceTranscript::Rejected,
        VoiceTranscript::Accepted {
            text: SpeechText::try_from(String::from("synthetic spoken text")).unwrap(),
            confidence: Confidence::Accepted,
        },
    ] {
        let (mut provider, observed, _calls) = fake();
        provider.reply = Some(Ok(transcript.clone()));
        let result = Finalization::new(utterance(), &generation)
            .unwrap()
            .run(&mut provider)
            .await
            .unwrap();
        assert_eq!(result, transcript);
        assert!(observed.await.unwrap().is_cancelled());
    }
    let (mut provider, _observed, _calls) = fake();
    provider.reply = Some(Err(SttError::Endpoint(EndpointError::Rejected)));
    let error = Finalization::new(utterance(), &generation)
        .unwrap()
        .run(&mut provider)
        .await
        .unwrap_err();
    assert!(matches!(error, Failure::Transcription(_)));
    assert!(!format!("{error:?}").contains("synthetic spoken text"));
}
