//! Synthetic providers and loopback only; events establish each ordering edge.

use super::super::{provider_json::DecodeError, text::Text};
use super::*;
use std::time::Duration;
use vessel_proto::dictation::DictationPcm;

const TEST_LIMIT: Duration = Duration::from_secs(60);

fn segment() -> Segment {
    Segment::from_test_pcm(DictationPcm::new(1000_i16.to_le_bytes().to_vec()).unwrap())
}
fn text(value: &str) -> Transcript {
    Transcript::try_from(value.to_owned()).unwrap()
}
fn fake() -> (
    Jobs,
    mpsc::Receiver<FakeRequest>,
    watch::Sender<CommitPhase>,
) {
    let (sender, requests) = mpsc::channel(super::super::MAX_IN_FLIGHT_COMMITS);
    let (phase, receiver) = watch::channel(CommitPhase::Running);
    (Jobs::new(Source::Fake(sender), receiver), requests, phase)
}

#[tokio::test]
async fn later_completion_releases_a_slot_without_waiting_for_an_earlier_result() {
    vessel_testkit::within(TEST_LIMIT, async {
        let (mut jobs, mut requests, _phase) = fake();
        let mut slots = Text::default();
        let first = slots.reserve().unwrap();
        let second = slots.reserve().unwrap();
        jobs.spawn(first, segment()).unwrap();
        let first_request = requests.recv().await.unwrap();
        jobs.spawn(second, segment()).unwrap();
        let second_request = requests.recv().await.unwrap();
        assert!(!first_request.segment.as_slice().is_empty());
        let third = slots.reserve().unwrap();
        second_request.reply.send(Ok(text("second"))).unwrap();
        let completed = jobs.take().await.unwrap().unwrap();
        assert_eq!(completed.index, second);
        assert!(completed.result.unwrap().as_str() == "second");
        assert!(!jobs.full());
        jobs.spawn(third, segment()).unwrap();
        let third_request = requests.recv().await.unwrap();
        third_request.reply.send(Ok(text("third"))).unwrap();
        assert_eq!(jobs.take().await.unwrap().unwrap().index, third);
        first_request.reply.send(Ok(text("first"))).unwrap();
        assert_eq!(jobs.take().await.unwrap().unwrap().index, first);
        assert!(jobs.empty());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn phase_is_captured_at_provider_return_even_when_collection_waits_for_finish() {
    vessel_testkit::within(TEST_LIMIT, async {
        let (mut jobs, mut requests, phase) = fake();
        let (observed, mut observations) = watch::channel(None);
        jobs.observed = Some(observed);
        let mut slots = Text::default();
        let first = slots.reserve().unwrap();
        jobs.spawn(first, segment()).unwrap();
        requests
            .recv()
            .await
            .unwrap()
            .reply
            .send(Err(ProviderError::Decode(DecodeError::TooSmall)))
            .unwrap();
        observations.changed().await.unwrap();
        assert_eq!(
            *observations.borrow_and_update(),
            Some((first, CommitPhase::Running))
        );
        assert_eq!(
            phase.send_replace(CommitPhase::Finishing),
            CommitPhase::Running
        );
        let completed = jobs.take().await.unwrap().unwrap();
        assert_eq!(completed.phase, CommitPhase::Running);
        assert!(matches!(
            completed.result,
            Err(JobError::Endpoint(ProviderError::Decode(
                DecodeError::TooSmall
            )))
        ));
        let second = slots.reserve().unwrap();
        jobs.spawn(second, segment()).unwrap();
        requests
            .recv()
            .await
            .unwrap()
            .reply
            .send(Err(ProviderError::Decode(DecodeError::TooSmall)))
            .unwrap();
        let completed = jobs.take().await.unwrap().unwrap();
        assert_eq!(completed.phase, CommitPhase::Finishing);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn clearing_waits_until_provider_futures_are_dropped() {
    vessel_testkit::within(TEST_LIMIT, async {
        let (mut jobs, mut requests, _phase) = fake();
        let mut slots = Text::default();
        jobs.spawn(slots.reserve().unwrap(), segment()).unwrap();
        let mut request = requests.recv().await.unwrap();
        jobs.clear().await.unwrap();
        request.reply.closed().await;
        assert!(jobs.empty());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn the_endpoint_job_uses_the_real_http_and_response_codecs() {
    vessel_testkit::within(TEST_LIMIT, async {
        use super::super::endpoint::tests::Server;
        use axum::{Json, Router, routing::post};
        let server = Server::start(Router::new().route(
            "/v1/audio/transcriptions",
            post(|| async { Json(serde_json::json!({"text":"recognized"})) }),
        ))
        .await;
        let (_phase, receiver) = watch::channel(CommitPhase::Running);
        let mut jobs = Jobs::new(Source::Endpoint(server.endpoint().await), receiver);
        let mut slots = Text::default();
        jobs.spawn(slots.reserve().unwrap(), segment()).unwrap();
        let result = jobs.take().await.unwrap().unwrap().result.unwrap();
        assert!(result.as_str() == "recognized");
        server.finish().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_closed_synthetic_provider_keeps_the_channel_cause_without_payload() {
    vessel_testkit::within(TEST_LIMIT, async {
        let (mut jobs, requests, _phase) = fake();
        drop(requests);
        let mut slots = Text::default();
        jobs.spawn(slots.reserve().unwrap(), segment()).unwrap();
        let error = jobs.take().await.unwrap().unwrap().result.unwrap_err();
        assert!(matches!(&error, JobError::Send(_)));
        assert!(std::error::Error::source(&error).is_some());
        assert_eq!(format!("{error:?}"), error.to_string());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_full_pool_returns_ownership_without_starting_an_extra_provider() {
    vessel_testkit::within(TEST_LIMIT, async {
        let (mut jobs, mut requests, _phase) = fake();
        let mut slots = Text::default();
        let mut held = Vec::new();
        for _ in 0..super::super::MAX_IN_FLIGHT_COMMITS {
            jobs.spawn(slots.reserve().unwrap(), segment()).unwrap();
            held.push(requests.recv().await.unwrap());
        }
        assert!(jobs.full());
        let index = slots.reserve().unwrap();
        let input = segment();
        let expected = input.clone();
        let refused = jobs.spawn(index, input).unwrap_err();
        assert_eq!(refused.index, index);
        assert!(
            refused.segment.as_slice() == expected.as_slice(),
            "returned segment changed"
        );
        assert_eq!(jobs.tasks.len(), super::super::MAX_IN_FLIGHT_COMMITS);
        assert!(matches!(
            requests.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        jobs.clear().await.unwrap();
        assert!(jobs.empty());
        drop(held);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_worker_panic_keeps_its_typed_cause_out_of_debug_output() {
    vessel_testkit::within(TEST_LIMIT, async {
        let (mut jobs, _requests, _phase) = fake();
        let _abort = jobs
            .tasks
            .spawn(async { panic!("synthetic-private-worker-panic") });
        let error = match jobs.take().await.unwrap() {
            Ok(_) => panic!("a panicked worker returned a completion"),
            Err(error) => error,
        };
        match &error {
            JobError::Join(cause) => assert!(cause.is_panic()),
            JobError::Endpoint(_) | JobError::Send(_) | JobError::Reply(_) => panic!("wrong cause"),
        }
        assert!(std::error::Error::source(&error).is_some());
        assert!(!format!("{error:?}").contains("synthetic-private-worker-panic"));
        assert_eq!(format!("{error:?}"), error.to_string());
        assert!(jobs.empty());
    })
    .await
    .unwrap();
}
