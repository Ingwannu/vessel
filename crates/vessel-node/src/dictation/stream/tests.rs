//! Real stream scheduling against a bounded, explicitly completed provider.

use super::*;
use crate::dictation::{
    audio::{COMMIT_SECONDS, PROVIDER_HZ},
    endpoint::ProviderError,
    jobs::FakeRequest,
    provider_json::DecodeError,
    updates::{End, Event, ReadRefusal},
};
use vessel_proto::dictation::{ChunkSeq, DictationPcm, DictationReason, Transcript};

const TEST_LIMIT: Duration = Duration::from_secs(60);
struct Fixture {
    handle: Handle,
    task: JoinHandle<()>,
    requests: mpsc::Receiver<FakeRequest>,
    owner: CancellationToken,
    node: CancellationToken,
}
impl Fixture {
    fn new() -> Self {
        Self::with_owner(CancellationToken::new())
    }
    fn with_owner(owner: CancellationToken) -> Self {
        let node = CancellationToken::new();
        let (source, requests) = mpsc::channel(super::super::MAX_IN_FLIGHT_COMMITS);
        let (handle, task) = spawn(
            SampleRate::new(PROVIDER_HZ).unwrap(),
            Source::Fake(source),
            CloseSignals {
                owner: owner.clone(),
                node: node.clone(),
            },
            pending(),
            Control::new(&owner),
            crate::dictation::quota::tests::lease(),
        )
        .unwrap();
        Self {
            handle,
            task,
            requests,
            owner,
            node,
        }
    }
    async fn request(&mut self) -> FakeRequest {
        tokio::time::timeout(TEST_LIMIT, self.requests.recv())
            .await
            .unwrap()
            .unwrap()
    }
    async fn stop(self) {
        self.handle.cancel().await;
        self.task.await.unwrap();
    }
}
fn text(value: &str) -> Transcript {
    Transcript::try_from(value.to_owned()).unwrap()
}
fn pcm(sample: i16) -> DictationPcm {
    DictationPcm::new(
        sample
            .to_le_bytes()
            .repeat(usize::try_from(PROVIDER_HZ).unwrap()),
    )
    .unwrap()
}
async fn feed(handle: &Handle, start: usize, count: usize, sample: i16) {
    for index in start..start.checked_add(count).unwrap() {
        let seq = ChunkSeq::new(u32::try_from(index).unwrap());
        assert_eq!(handle.chunk(seq, pcm(sample)).await.unwrap(), Some(seq));
    }
}
async fn partial(handle: &Handle, wanted: &str) {
    match handle.next().await.unwrap() {
        Event::Partial(text) => assert!(text.as_str() == wanted),
        Event::End(_) => panic!("expected a partial"),
    }
}
async fn final_text(handle: &Handle, wanted: &str) {
    match handle.next().await.unwrap() {
        Event::End(End::Final(text)) => assert!(text.as_str() == wanted),
        Event::Partial(_) | Event::End(End::Failed(_)) => panic!("expected a final"),
    }
}
async fn failed(handle: &Handle, wanted: DictationReason) {
    match handle.next().await.unwrap() {
        Event::End(End::Failed(error)) => assert_eq!(error.reason(), Some(wanted)),
        Event::Partial(_) | Event::End(End::Final(_)) => panic!("expected a failure"),
    }
}

#[tokio::test]
async fn later_completion_is_visible_before_the_first_and_final_uses_submission_order() {
    tokio::time::timeout(TEST_LIMIT, async {
        let mut f = Fixture::new();
        feed(&f.handle, 0, COMMIT_SECONDS, 1000).await;
        let first = f.request().await;
        feed(&f.handle, COMMIT_SECONDS, COMMIT_SECONDS, 2000).await;
        let second = f.request().await;
        assert_ne!(first.segment.as_slice(), second.segment.as_slice());
        second.reply.send(Ok(text("second"))).unwrap();
        partial(&f.handle, "second").await;
        first.reply.send(Ok(text("first"))).unwrap();
        partial(&f.handle, "first second").await;
        let last = ChunkSeq::new(u32::try_from(COMMIT_SECONDS * 2 - 1).unwrap());
        let _budget = f.handle.finish(last).await.unwrap();
        final_text(&f.handle, "first second").await;
        f.task.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_short_tail_is_acked_early_and_sealed_before_the_finish_estimate() {
    tokio::time::timeout(TEST_LIMIT, async {
        let mut f = Fixture::new();
        let seq = ChunkSeq::new(0);
        assert_eq!(
            f.handle
                .chunk(
                    seq,
                    DictationPcm::new(1000_i16.to_le_bytes().to_vec()).unwrap()
                )
                .await
                .unwrap(),
            Some(seq)
        );
        assert!(matches!(
            f.requests.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        let within = f.handle.finish(seq).await.unwrap();
        let expected = crate::dictation::budget::compute(
            crate::dictation::budget::PendingCommits::new(1),
            Duration::ZERO,
            Some(seq),
            seq,
        )
        .unwrap();
        assert_eq!(within, expected);
        f.request().await.reply.send(Ok(text("tail"))).unwrap();
        partial(&f.handle, "tail").await;
        final_text(&f.handle, "tail").await;
        f.task.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn finishing_too_small_settles_without_a_fabricated_partial() {
    tokio::time::timeout(TEST_LIMIT, async {
        let mut f = Fixture::new();
        assert_eq!(
            f.handle.chunk(ChunkSeq::new(0), pcm(1000)).await.unwrap(),
            Some(ChunkSeq::new(0))
        );
        let _budget = f.handle.finish(ChunkSeq::new(0)).await.unwrap();
        f.request()
            .await
            .reply
            .send(Err(ProviderError::Decode(DecodeError::TooSmall)))
            .unwrap();
        final_text(&f.handle, "").await;
        f.task.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn too_small_before_finish_is_a_failure_and_an_empty_success_emits_partial() {
    tokio::time::timeout(TEST_LIMIT, async {
        let mut f = Fixture::new();
        feed(&f.handle, 0, COMMIT_SECONDS, 1000).await;
        f.request()
            .await
            .reply
            .send(Err(ProviderError::Decode(DecodeError::TooSmall)))
            .unwrap();
        failed(&f.handle, DictationReason::Upstream).await;
        f.task.await.unwrap();
        let mut f = Fixture::new();
        feed(&f.handle, 0, COMMIT_SECONDS, 1000).await;
        f.request().await.reply.send(Ok(text(""))).unwrap();
        partial(&f.handle, "").await;
        f.stop().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn silence_finishes_without_sending_and_no_audio_refuses_pending_next() {
    tokio::time::timeout(TEST_LIMIT, async {
        let mut f = Fixture::new();
        assert_eq!(
            f.handle.chunk(ChunkSeq::new(0), pcm(0)).await.unwrap(),
            Some(ChunkSeq::new(0))
        );
        let _budget = f.handle.finish(ChunkSeq::new(0)).await.unwrap();
        final_text(&f.handle, "").await;
        f.task.await.unwrap();
        assert!(matches!(
            f.requests.try_recv(),
            Err(mpsc::error::TryRecvError::Disconnected)
        ));
        let f = Fixture::new();
        let (reply, response) = oneshot::channel();
        f.handle.next.send(reply).await.unwrap();
        let error = f.handle.finish(ChunkSeq::new(0)).await.unwrap_err();
        assert_eq!(error.reason(), Some(DictationReason::NoAudio));
        assert!(matches!(
            response.await.unwrap(),
            Reply::Refused(ReadRefusal::Gone)
        ));
        f.task.await.unwrap();
    })
    .await
    .unwrap();
}

#[path = "control_tests.rs"]
mod control;

#[path = "backpressure_tests.rs"]
mod backpressure;

#[tokio::test]
async fn preparation_control_and_actual_actor_lease_survive_the_spawn_boundary() {
    tokio::time::timeout(TEST_LIMIT, async {
        let owner = CancellationToken::new();
        let control = Control::new(&owner);
        let first = Arc::new(StreamError::Input(
            crate::dictation::input::InputError::TooLong,
        ));
        let selected = control.stop(
            Arc::clone(&first),
            super::super::control::Retention::KeepTerminal,
        );
        assert!(Arc::ptr_eq(&first, &selected));
        let lease = crate::dictation::quota::tests::lease();
        let retained = Arc::downgrade(&lease);
        let (provider, _requests) = mpsc::channel(super::super::MAX_IN_FLIGHT_COMMITS);
        let (handle, task) = spawn(
            SampleRate::TRANSCRIPTION,
            Source::Fake(provider),
            CloseSignals {
                owner,
                node: CancellationToken::new(),
            },
            pending(),
            control,
            lease,
        )
        .unwrap();
        assert!(retained.upgrade().is_some());
        match handle.next().await.unwrap() {
            Event::End(End::Failed(error)) => assert!(Arc::ptr_eq(&error, &first)),
            Event::Partial(_) | Event::End(End::Final(_)) => {
                panic!("the selected failure was lost")
            }
        }
        task.await.unwrap();
        assert!(retained.upgrade().is_none());
    })
    .await
    .unwrap();
}
