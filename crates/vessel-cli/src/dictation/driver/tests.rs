//! Scripted Call/Reply at the client edge exercises the real file sender/state machine.
use super::*;
use crate::dictation::{
    client::Frame,
    file::tests::{file, format, wave},
};
use vessel_proto::{
    Ack, Call, ErrorCode, Reply, WireError,
    dictation::{ChunkSeq, DictationAck, DictationReason, FinishAccepted, SampleRate},
};

const TEST_LIMIT: Duration = Duration::from_secs(60);
async fn frame(receiver: &mut tokio::sync::mpsc::Receiver<Frame>) -> Frame {
    receiver.recv().await.unwrap()
}
fn answer(frame: Frame, reply: Reply) {
    frame.reply.send(Ok(reply)).unwrap();
}
fn input_file() -> (tempfile::TempDir, std::path::PathBuf) {
    let rate = SampleRate::MIN;
    let audio = 1000_i16
        .to_le_bytes()
        .repeat(usize::try_from(rate.get()).unwrap().checked_add(1).unwrap());
    file(&wave(&[(b"fmt ", &format(rate)), (b"data", &audio)]))
}
fn transcript() -> Transcript {
    Transcript::try_from("recognized".to_owned()).unwrap()
}

async fn chunk_and_next(receiver: &mut tokio::sync::mpsc::Receiver<Frame>) -> (Frame, Frame) {
    let mut chunk = None;
    let mut next = None;
    for _ in 0..2 {
        let current = frame(receiver).await;
        if matches!(current.call, Call::ChunkDictation(_)) {
            chunk = Some(current);
        } else {
            assert!(matches!(current.call, Call::NextDictation(_)));
            next = Some(current);
        }
    }
    (chunk.unwrap(), next.unwrap())
}

#[tokio::test]
async fn one_second_plus_tail_drains_partials_and_holds_an_early_final_until_finish_acceptance() {
    vessel_testkit::within(TEST_LIMIT, async {
        let (_root, path) = input_file();
        let input = FileInput::open(&path).await.unwrap();
        let (client, mut requests) = Client::scripted();
        let id = DictationId::random();
        let never = pending::<io::Result<()>>(); tokio::pin!(never);
        let mut running = Box::pin(transcribe(&client, &id, input, never.as_mut()));
        let script = async {
            let start = frame(&mut requests).await;
            let Call::StartDictation(request) = &start.call else { panic!("expected start"); };
            assert_eq!(request.id, id);
            answer(start, Reply::StartDictation(Ack));
            let (first, next) = chunk_and_next(&mut requests).await;
            let Call::ChunkDictation(request) = &first.call else { panic!("expected chunk zero"); };
            assert_eq!(request.seq, ChunkSeq::new(0));
            assert_eq!(request.pcm.as_slice().len(), usize::try_from(SampleRate::MIN.get()).unwrap().checked_mul(size_of::<i16>()).unwrap());
            answer(next, Reply::NextDictation(DictationUpdate::Partial { text: transcript() }));
            answer(first, Reply::ChunkDictation(DictationAck { through: Some(ChunkSeq::new(0)) }));
            let (tail, next) = chunk_and_next(&mut requests).await;
            let Call::ChunkDictation(request) = &tail.call else { panic!("expected the short tail"); };
            assert_eq!(request.seq, ChunkSeq::new(1));
            assert_eq!(request.pcm.as_slice().len(), size_of::<i16>());
            answer(tail, Reply::ChunkDictation(DictationAck { through: Some(ChunkSeq::new(1)) }));
            let finish = frame(&mut requests).await;
            let Call::FinishDictation(request) = &finish.call else { panic!("expected finish"); };
            assert_eq!(request.final_seq, ChunkSeq::new(1));
            answer(next, Reply::NextDictation(DictationUpdate::Final { text: transcript() }));
            finish
        };
        let finish = tokio::select! {
            result = running.as_mut() => panic!("command finished before its acknowledgement: {}", result.is_ok()),
            finish = script => finish,
        };
        assert!(futures_util::poll!(running.as_mut()).is_pending());
        answer(finish, Reply::FinishDictation(FinishAccepted { within_ms: WithinMs::MAX }));
        assert!(running.await.unwrap().as_str() == "recognized");
        assert!(requests.try_recv().is_err());
    }).await.unwrap();
}

#[tokio::test]
async fn a_stream_failure_is_not_resent_and_cleanup_failure_does_not_replace_its_reason() {
    vessel_testkit::within(TEST_LIMIT, async {
        let (_root, path) = input_file();
        let input = FileInput::open(&path).await.unwrap();
        let (client, mut requests) = Client::scripted();
        let id = DictationId::random();
        let never = pending::<io::Result<()>>();
        tokio::pin!(never);
        let running = transcribe(&client, &id, input, never.as_mut());
        let script = async {
            let start = frame(&mut requests).await;
            answer(start, Reply::StartDictation(Ack));
            let (chunk, next) = chunk_and_next(&mut requests).await;
            answer(
                next,
                Reply::NextDictation(DictationUpdate::Failed {
                    reason: DictationReason::Upstream,
                }),
            );
            let cancel = frame(&mut requests).await;
            assert!(matches!(cancel.call, Call::CancelDictation(_)));
            assert!(chunk.reply.is_closed());
            cancel
                .reply
                .send(Err(CallError::Remote(WireError::new(
                    ErrorCode::Internal,
                    "private-cleanup-marker",
                ))))
                .unwrap();
        };
        let (result, ()) = tokio::join!(running, script);
        let error = result.unwrap_err();
        assert!(error.to_string().contains("audio was not resent"));
        assert!(
            error
                .to_string()
                .contains("cancellation could not be confirmed")
        );
        assert!(!error.to_string().contains("private-cleanup-marker"));
        assert!(requests.try_recv().is_err());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cancellation_while_upload_and_next_are_pending_uses_one_cancel_and_no_finish() {
    vessel_testkit::within(TEST_LIMIT, async {
        let (_root, path) = input_file();
        let input = FileInput::open(&path).await.unwrap();
        let (client, mut requests) = Client::scripted();
        let id = DictationId::random();
        let stop = tokio_util::sync::CancellationToken::new();
        let observe = stop.clone();
        let interrupt = async move {
            observe.cancelled().await;
            Ok(())
        };
        tokio::pin!(interrupt);
        let running = transcribe(&client, &id, input, interrupt.as_mut());
        let script = async {
            answer(frame(&mut requests).await, Reply::StartDictation(Ack));
            let first = frame(&mut requests).await;
            let second = frame(&mut requests).await;
            stop.cancel();
            let cancel = frame(&mut requests).await;
            assert!(matches!(cancel.call, Call::CancelDictation(_)));
            assert!(first.reply.is_closed());
            assert!(second.reply.is_closed());
            answer(cancel, Reply::CancelDictation(Ack));
        };
        let (result, ()) = tokio::join!(running, script);
        assert!(result.unwrap_err().to_string().contains("cancelled"));
        assert!(requests.try_recv().is_err());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_bad_ack_is_a_refusal_instead_of_a_resend() {
    vessel_testkit::within(TEST_LIMIT, async {
        let (_root, path) = input_file();
        let input = FileInput::open(&path).await.unwrap();
        let (client, mut requests) = Client::scripted();
        let id = DictationId::random();
        let never = pending::<io::Result<()>>();
        tokio::pin!(never);
        let running = transcribe(&client, &id, input, never.as_mut());
        let script = async {
            answer(frame(&mut requests).await, Reply::StartDictation(Ack));
            let (chunk, next) = chunk_and_next(&mut requests).await;
            answer(chunk, Reply::ChunkDictation(DictationAck { through: None }));
            let cancel = frame(&mut requests).await;
            assert!(matches!(cancel.call, Call::CancelDictation(_)));
            assert!(next.reply.is_closed());
            answer(cancel, Reply::CancelDictation(Ack));
        };
        let (result, ()) = tokio::join!(running, script);
        assert!(result.unwrap_err().to_string().contains("acknowledge"));
        assert!(requests.try_recv().is_err());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn truncation_after_one_ack_is_an_input_failure_and_never_a_short_success() {
    vessel_testkit::within(TEST_LIMIT, async {
        let (_root, path) = input_file();
        let input = FileInput::open(&path).await.unwrap();
        let (client, mut requests) = Client::scripted();
        let id = DictationId::random();
        let never = pending::<io::Result<()>>();
        tokio::pin!(never);
        let running = transcribe(&client, &id, input, never.as_mut());
        let script = async {
            answer(frame(&mut requests).await, Reply::StartDictation(Ack));
            let (chunk, next) = chunk_and_next(&mut requests).await;
            let file = tokio::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .await
                .unwrap();
            file.set_len(0).await.unwrap();
            answer(
                chunk,
                Reply::ChunkDictation(DictationAck {
                    through: Some(ChunkSeq::new(0)),
                }),
            );
            let cancel = frame(&mut requests).await;
            assert!(matches!(cancel.call, Call::CancelDictation(_)));
            assert!(next.reply.is_closed());
            answer(cancel, Reply::CancelDictation(Ack));
        };
        let (result, ()) = tokio::join!(running, script);
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("input could not be read")
        );
        assert!(requests.try_recv().is_err());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn unknown_update_and_final_before_finish_are_refused_with_one_cancel() {
    for update in [
        DictationUpdate::Unknown,
        DictationUpdate::Final { text: transcript() },
    ] {
        vessel_testkit::within(TEST_LIMIT, async {
            let (_root, path) = input_file();
            let input = FileInput::open(&path).await.unwrap();
            let (client, mut requests) = Client::scripted();
            let id = DictationId::random();
            let never = pending::<io::Result<()>>();
            tokio::pin!(never);
            let running = transcribe(&client, &id, input, never.as_mut());
            let script = async {
                answer(frame(&mut requests).await, Reply::StartDictation(Ack));
                let (chunk, next) = chunk_and_next(&mut requests).await;
                answer(next, Reply::NextDictation(update));
                let cancel = frame(&mut requests).await;
                assert!(matches!(cancel.call, Call::CancelDictation(_)));
                assert!(chunk.reply.is_closed());
                answer(cancel, Reply::CancelDictation(Ack));
            };
            let (result, ()) = tokio::join!(running, script);
            assert!(result.is_err());
            assert!(requests.try_recv().is_err());
        })
        .await
        .unwrap();
    }
}

// Complete all file I/O on the real clock before a wait-only test pauses time.
async fn upload_until_finish(receiver: &mut tokio::sync::mpsc::Receiver<Frame>) -> (Frame, Frame) {
    answer(frame(receiver).await, Reply::StartDictation(Ack));
    let (first, next) = chunk_and_next(receiver).await;
    answer(
        first,
        Reply::ChunkDictation(DictationAck {
            through: Some(ChunkSeq::new(0)),
        }),
    );
    let tail = frame(receiver).await;
    assert!(matches!(tail.call, Call::ChunkDictation(_)));
    answer(
        tail,
        Reply::ChunkDictation(DictationAck {
            through: Some(ChunkSeq::new(1)),
        }),
    );
    let finish = frame(receiver).await;
    assert!(matches!(finish.call, Call::FinishDictation(_)));
    (finish, next)
}

#[tokio::test]
async fn finish_acceptance_wait_is_bounded_and_abandons_the_pending_final() {
    let (_root, path) = input_file();
    let input = FileInput::open(&path).await.unwrap();
    let (client, mut requests) = Client::scripted();
    let id = DictationId::random();
    let never = pending::<io::Result<()>>();
    tokio::pin!(never);
    let mut running = Box::pin(transcribe(&client, &id, input, never.as_mut()));
    let (finish, next) = vessel_testkit::within(TEST_LIMIT, async {
        tokio::select! {
            result = running.as_mut() => panic!("unexpected early completion: {}", result.is_ok()),
            pair = upload_until_finish(&mut requests) => pair,
        }
    })
    .await
    .unwrap();
    tokio::time::pause();
    assert!(futures_util::poll!(running.as_mut()).is_pending());
    tokio::time::advance(Duration::from_millis(u64::from(WithinMs::MAX.get()))).await;
    let cleanup = async {
        let cancel = frame(&mut requests).await;
        assert!(matches!(cancel.call, Call::CancelDictation(_)));
        assert!(finish.reply.is_closed());
        assert!(next.reply.is_closed());
        answer(cancel, Reply::CancelDictation(Ack));
    };
    let (result, ()) = vessel_testkit::within(CANCEL_TIMEOUT.saturating_mul(2), async {
        tokio::join!(running, cleanup)
    })
    .await
    .unwrap();
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("acceptance exceeded")
    );
    assert!(requests.try_recv().is_err());
}

#[tokio::test]
async fn returned_final_budget_and_cleanup_budget_do_not_wait_indefinitely() {
    const RETURNED_BUDGET: WithinMs = WithinMs::new(7);
    let (_root, path) = input_file();
    let input = FileInput::open(&path).await.unwrap();
    let (client, mut requests) = Client::scripted();
    let id = DictationId::random();
    let never = pending::<io::Result<()>>();
    tokio::pin!(never);
    let mut running = Box::pin(transcribe(&client, &id, input, never.as_mut()));
    let (finish, next) = vessel_testkit::within(TEST_LIMIT, async {
        tokio::select! {
            result = running.as_mut() => panic!("unexpected early completion: {}", result.is_ok()),
            pair = upload_until_finish(&mut requests) => pair,
        }
    })
    .await
    .unwrap();
    tokio::time::pause();
    answer(
        finish,
        Reply::FinishDictation(FinishAccepted {
            within_ms: RETURNED_BUDGET,
        }),
    );
    assert!(futures_util::poll!(running.as_mut()).is_pending());
    tokio::time::advance(Duration::from_millis(u64::from(RETURNED_BUDGET.get()))).await;
    let cancel = tokio::select! {
        result = running.as_mut() => panic!("cleanup was not attempted: {}", result.is_ok()),
        cancel = frame(&mut requests) => cancel,
    };
    assert!(matches!(cancel.call, Call::CancelDictation(_)));
    assert!(next.reply.is_closed());
    tokio::time::advance(CANCEL_TIMEOUT).await;
    let error = running.await.unwrap_err();
    assert!(cancel.reply.is_closed());
    assert!(error.to_string().contains("final transcript exceeded"));
    assert!(
        error
            .to_string()
            .contains("cancellation could not be confirmed")
    );
    assert!(requests.try_recv().is_err());
}

#[tokio::test]
async fn local_protocol_admission_refuses_start_without_an_unnecessary_cancel() {
    for source in [
        CallError::UnassignedProtocol {
            method: Method::StartDictation,
        },
        CallError::NotInProtocol {
            method: Method::StartDictation,
            needs: vessel_proto::PROTOCOL_VERSION,
            negotiated: vessel_proto::MIN_PROTOCOL_VERSION,
        },
    ] {
        vessel_testkit::within(TEST_LIMIT, async {
            let (_root, path) = input_file();
            let input = FileInput::open(&path).await.unwrap();
            let (client, mut requests) = Client::scripted();
            let id = DictationId::random();
            let never = pending::<io::Result<()>>();
            tokio::pin!(never);
            let running = transcribe(&client, &id, input, never.as_mut());
            let script = async {
                let start = frame(&mut requests).await;
                assert!(matches!(start.call, Call::StartDictation(_)));
                start.reply.send(Err(source)).unwrap();
            };
            let (result, ()) = tokio::join!(running, script);
            assert!(result.is_err());
            assert!(requests.try_recv().is_err());
        })
        .await
        .unwrap();
    }
}
