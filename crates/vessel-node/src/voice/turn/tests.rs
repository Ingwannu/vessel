use super::*;
use crate::voice::{finalize::Transcriber, input_buffer::Utterance, stt_endpoint::SttError};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use vessel_proto::voice::{
    CapturePcm, Confidence, SpeechText, VoiceEpoch, VoiceId, VoiceTranscript,
};

#[derive(Debug, thiserror::Error)]
#[error("synthetic foreign tensor contents must not enter diagnostics")]
struct InferenceFailure;
struct Scores {
    calls: Arc<AtomicUsize>,
    resets: Arc<AtomicUsize>,
    fail_at: Option<usize>,
}
impl Detector for Scores {
    type Error = InferenceFailure;
    fn probability(&mut self, window: &Window) -> Result<Probability, Self::Error> {
        if window.samples().len() != 512 {
            return Err(InferenceFailure);
        }
        let index = self.calls.fetch_add(1, Ordering::AcqRel);
        if self.fail_at == Some(index) {
            return Err(InferenceFailure);
        }
        // Thirty high windows confirm the first speech; forty low windows end
        // it. A subsequent high run is a genuine new onset, not an RMS rule.
        Probability::new(if (30..70).contains(&index) { 0.0 } else { 1.0 })
            .map_err(|_private| InferenceFailure)
    }
    fn reset(&mut self) {
        let _old = self.resets.fetch_add(1, Ordering::AcqRel);
    }
}
fn setup() -> (
    Turn<Scores>,
    CancellationToken,
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
) {
    let parent = CancellationToken::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let resets = Arc::new(AtomicUsize::new(0));
    let turn = Turn::new(
        lease(),
        &parent,
        Scores {
            calls: Arc::clone(&calls),
            resets: Arc::clone(&resets),
            fail_at: None,
        },
    );
    (turn, parent, calls, resets)
}
fn lease() -> VoiceLease {
    VoiceLease {
        id: VoiceId::random(),
        epoch: VoiceEpoch::random(),
    }
}
fn packet(turn: &Turn<Scores>, sequence: u64) -> CaptureVoice {
    CaptureVoice {
        lease: turn.lease,
        seq: VoiceSeq::new(sequence),
        pcm: CapturePcm::new(vec![0; 1_024]).unwrap(),
    }
}
fn append(turn: &mut Turn<Scores>, sequence: u64) -> Transition {
    let result = turn.append(&packet(turn, sequence)).unwrap();
    assert_eq!(result.ack.through, VoiceSeq::new(sequence));
    result.transition
}
fn first_finalization(turn: &mut Turn<Scores>) -> Finalization {
    for sequence in 0..=70 {
        match append(turn, sequence) {
            Transition::None => {}
            Transition::SpeechStarted => assert_eq!(sequence, 30),
            Transition::Finalizing(finalization) => {
                assert_eq!(sequence, 70);
                return finalization;
            }
        }
    }
    panic!("ending boundary must create final STT")
}
struct Transcript {
    bytes: usize,
    calls: usize,
}
impl Transcriber for Transcript {
    fn recognize(
        &mut self,
        utterance: &Utterance,
        _cancel: &CancellationToken,
    ) -> impl Future<Output = Result<VoiceTranscript, SttError>> + Send {
        self.calls = self.calls.saturating_add(1);
        self.bytes = utterance.pcm16().len();
        std::future::ready(Ok(VoiceTranscript::Accepted {
            text: SpeechText::try_from(String::from("synthetic accepted speech")).unwrap(),
            confidence: Confidence::Accepted,
        }))
    }
}

#[tokio::test]
async fn ordered_pcm_reaches_the_real_boundaries_buffer_and_final_stt_once() {
    let (mut turn, parent, calls, _resets) = setup();
    let finalization = first_finalization(&mut turn);
    assert_eq!(calls.load(Ordering::Acquire), 70);
    let mut transcript = Transcript { bytes: 0, calls: 0 };
    assert!(matches!(
        finalization.run(&mut transcript).await.unwrap(),
        VoiceTranscript::Accepted { .. }
    ));
    assert_eq!(transcript.calls, 1);
    assert_eq!(transcript.bytes, 71 * 1_024);
    assert!(matches!(
        turn.append(&packet(&turn, 70)),
        Err(Failure::Sequence)
    ));
    assert_eq!(calls.load(Ordering::Acquire), 70);
    assert!(!parent.is_cancelled());
}

#[tokio::test(start_paused = true)]
async fn the_next_onset_cancels_a_queued_old_finalization_without_opening_stt() {
    let (mut turn, parent, _calls, _resets) = setup();
    let old = first_finalization(&mut turn);
    let mut starts = 0;
    for sequence in 71..=100 {
        if matches!(append(&mut turn, sequence), Transition::SpeechStarted) {
            starts += 1;
        }
    }
    assert_eq!(starts, 1);
    let mut transcript = Transcript { bytes: 0, calls: 0 };
    assert!(matches!(
        old.run(&mut transcript).await,
        Err(crate::voice::finalize::Failure::Cancelled)
    ));
    assert_eq!(transcript.calls, 0);
    assert!(!parent.is_cancelled());
}

#[test]
fn wrong_generation_duplicates_and_gaps_do_not_consume_pcm_or_detector_state() {
    let (mut turn, _parent, calls, _resets) = setup();
    let mut request = packet(&turn, 0);
    request.lease = lease();
    assert!(matches!(turn.append(&request), Err(Failure::Lease)));
    request.lease = turn.lease;
    request.seq = VoiceSeq::new(1);
    assert!(matches!(turn.append(&request), Err(Failure::Sequence)));
    assert_eq!(calls.load(Ordering::Acquire), 0);
    assert!(matches!(append(&mut turn, 0), Transition::None));
    assert!(matches!(
        turn.append(&packet(&turn, 0)),
        Err(Failure::Sequence)
    ));
    assert_eq!(calls.load(Ordering::Acquire), 0);
    assert!(matches!(append(&mut turn, 1), Transition::None));
    assert_eq!(calls.load(Ordering::Acquire), 1);
}

#[test]
fn inference_failure_clears_every_retained_sample_and_never_calls_a_later_window() {
    let (mut turn, parent, calls, resets) = setup();
    turn.detector.fail_at = Some(0);
    let mut request = packet(&turn, 0);
    request.pcm = CapturePcm::new(vec![0; 3_072]).unwrap();
    let error = turn.append(&request).unwrap_err();
    assert!(matches!(error, Failure::Detector));
    assert!(!format!("{error:?} {error}").contains("tensor"));
    assert!(std::error::Error::source(&error).is_none());
    assert_eq!(calls.load(Ordering::Acquire), 1);
    assert_eq!(resets.load(Ordering::Acquire), 1);
    assert!(turn.buffer.seal().is_none());
    assert!(turn.cancel.is_cancelled());
    assert!(matches!(turn.append(&request), Err(Failure::Stopped)));
    assert_eq!(calls.load(Ordering::Acquire), 1);
    assert!(!parent.is_cancelled());
}

#[tokio::test]
async fn stop_and_drop_cancel_child_work_without_flushing_or_cancelling_the_parent() {
    let (mut turn, parent, calls, resets) = setup();
    assert!(matches!(append(&mut turn, 0), Transition::None));
    let child = turn.work.clone();
    turn.stop();
    assert!(child.is_cancelled());
    assert_eq!(calls.load(Ordering::Acquire), 0);
    assert!(turn.buffer.seal().is_none());
    assert_eq!(resets.load(Ordering::Acquire), 1);
    assert!(!parent.is_cancelled());
    let (mut turn, parent, _calls, _resets) = setup();
    let finalization = first_finalization(&mut turn);
    drop(turn);
    let mut transcript = Transcript { bytes: 0, calls: 0 };
    assert!(matches!(
        finalization.run(&mut transcript).await,
        Err(crate::voice::finalize::Failure::Cancelled)
    ));
    assert_eq!(transcript.calls, 0);
    assert!(!parent.is_cancelled());
}
