//! Stop races exercise the running actor rather than only its signal value.
use super::*;

#[tokio::test]
async fn cancel_first_drops_provider_work_and_leaves_another_stream_alive() {
    tokio::time::timeout(TEST_LIMIT, async {
        let mut first = Fixture::new();
        let second = Fixture::with_owner(first.owner.clone());
        feed(&first.handle, 0, COMMIT_SECONDS, 1000).await;
        let mut request = first.request().await;
        let (reply, response) = oneshot::channel();
        first.handle.next.send(reply).await.unwrap();
        let error = match first.handle.next().await {
            Err(error) => error,
            Ok(_) => panic!("extra next was admitted"),
        };
        assert!(matches!(
            error.as_ref(),
            StreamError::Read(ReadRefusal::Waiting)
        ));
        first.handle.cancel().await;
        match response.await.unwrap() {
            Reply::Update(Event::End(End::Failed(error))) => {
                assert_eq!(error.reason(), Some(DictationReason::Cancelled))
            }
            Reply::Update(Event::Partial(_) | Event::End(End::Final(_))) | Reply::Refused(_) => {
                panic!("missing cancel")
            }
        }
        request.reply.closed().await;
        assert!(!first.owner.is_cancelled());
        first.task.await.unwrap();
        assert_eq!(
            second.handle.chunk(ChunkSeq::new(0), pcm(0)).await.unwrap(),
            Some(ChunkSeq::new(0))
        );
        let _budget = second.handle.finish(ChunkSeq::new(0)).await.unwrap();
        final_text(&second.handle, "").await;
        second.task.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn an_already_selected_failure_survives_replace_cancel_and_close() {
    tokio::time::timeout(TEST_LIMIT, async {
        let mut f = Fixture::new();
        feed(&f.handle, 0, COMMIT_SECONDS, 1000).await;
        f.request()
            .await
            .reply
            .send(Err(ProviderError::KeyAbsent))
            .unwrap();
        // Observing the first selection forces failure-before-retirement.
        let mut selected = f.handle.control.subscribe();
        drop(selected.wait_for(Option::is_some).await.unwrap());
        let first = f.handle.control.chosen().unwrap();
        f.handle.replace().await;
        f.handle.cancel().await;
        f.owner.cancel();
        f.node.cancel();
        assert!(Arc::ptr_eq(&first, &f.handle.control.chosen().unwrap()));
        assert_eq!(first.reason(), Some(DictationReason::Upstream));
        f.task.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn transport_and_node_close_abort_pending_work_without_connection_guard_ownership() {
    tokio::time::timeout(TEST_LIMIT, async {
        let f = Fixture::new();
        f.node.cancel();
        f.task.await.unwrap();
        assert!(!f.owner.is_cancelled());
        let (closed, transport) = oneshot::channel();
        let (source, _requests) = mpsc::channel(super::super::super::MAX_IN_FLIGHT_COMMITS);
        let (handle, task) = spawn(
            SampleRate::try_from(PROVIDER_HZ).unwrap(),
            Source::Fake(source),
            CloseSignals {
                owner: CancellationToken::new(),
                node: CancellationToken::new(),
            },
            async move {
                let _closed = transport.await;
            },
        )
        .unwrap();
        closed.send(()).unwrap();
        task.await.unwrap();
        assert!(handle.closed.is_cancelled());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn repeated_finish_keeps_the_original_budget_and_conflicts_do_not_change_it() {
    tokio::time::timeout(TEST_LIMIT, async {
        let f = Fixture::new();
        assert_eq!(
            f.handle.chunk(ChunkSeq::new(0), pcm(1000)).await.unwrap(),
            Some(ChunkSeq::new(0))
        );
        let last = ChunkSeq::new(2);
        let first = f.handle.finish(last).await.unwrap();
        assert_eq!(f.handle.finish(last).await.unwrap(), first);
        let error = f.handle.finish(ChunkSeq::new(1)).await.unwrap_err();
        assert_eq!(error.reason(), Some(DictationReason::BadAudio));
        assert_eq!(f.handle.finish(last).await.unwrap(), first);
        f.stop().await;
    })
    .await
    .unwrap();
}
