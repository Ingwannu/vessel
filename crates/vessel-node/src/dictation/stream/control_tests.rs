//! Stop races exercise the running actor rather than only its signal value.
use super::*;

#[tokio::test]
async fn cancel_first_drops_provider_work_and_leaves_another_stream_alive() {
    tokio::time::timeout(TEST_LIMIT, async {
        let mut first = Fixture::new();
        let second = Fixture::with_owner(first.owner.clone());
        feed(&first.handle, 0, COMMIT_SECONDS, 1000).await;
        let request = first.request().await;
        let (reply, response) = oneshot::channel();
        first.handle.next.send(reply).await.unwrap();
        let Err(error) = first.handle.next().await else {
            panic!("extra next was admitted");
        };
        assert!(matches!(
            error.as_ref(),
            StreamError::Read(ReadRefusal::Waiting)
        ));
        let cancel = first.handle.cancel();
        tokio::pin!(cancel);
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(cancel.as_mut().poll(cx)))
                .await
                .is_pending()
        );
        // Select cancel synchronously, then make a provider failure ready before
        // the actor runs again: neither result scheduling order may relabel it.
        request.reply.send(Err(ProviderError::KeyAbsent)).unwrap();
        cancel.await;
        match response.await.unwrap() {
            Reply::Update(Event::End(End::Failed(error))) => {
                assert_eq!(error.reason(), Some(DictationReason::Cancelled));
            }
            Reply::Update(Event::Partial(_) | Event::End(End::Final(_))) | Reply::Refused(_) => {
                panic!("missing cancel")
            }
        }
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

#[tokio::test(start_paused = true)]
async fn an_already_selected_failure_survives_replace_cancel_and_close() {
    tokio::time::timeout(TEST_LIMIT, async {
        let mut f = Fixture::new();
        feed(&f.handle, 0, COMMIT_SECONDS, 1000).await;
        let within = f
            .handle
            .finish(ChunkSeq::new(u32::try_from(COMMIT_SECONDS).unwrap()))
            .await
            .unwrap();
        f.request()
            .await
            .reply
            .send(Err(ProviderError::KeyAbsent))
            .unwrap();
        // Observing the first selection forces failure-before-retirement.
        let mut selected = f.handle.control.subscribe();
        drop(selected.wait_for(Option::is_some).await.unwrap());
        let first = f.handle.control.chosen().unwrap();
        tokio::time::advance(
            Duration::from_millis(u64::from(within.get())) + Duration::from_millis(1),
        )
        .await;
        let replace = f.handle.replace();
        tokio::pin!(replace);
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(replace.as_mut().poll(cx)))
                .await
                .is_pending()
        );
        let cancel = f.handle.cancel();
        tokio::pin!(cancel);
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(cancel.as_mut().poll(cx)))
                .await
                .is_pending()
        );
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
            SampleRate::new(PROVIDER_HZ).unwrap(),
            Source::Fake(source),
            CloseSignals {
                owner: CancellationToken::new(),
                node: CancellationToken::new(),
            },
            async move {
                let _closed = transport.await;
            },
            Control::new(&CancellationToken::new()),
            crate::dictation::quota::tests::lease(),
        )
        .unwrap();
        closed.send(()).unwrap();
        task.await.unwrap();
        assert!(handle.closed.is_cancelled());
    })
    .await
    .unwrap();
}

#[tokio::test(start_paused = true)]
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
        let half = Duration::from_millis(u64::from(first.get()).div_euclid(2));
        tokio::time::advance(half).await;
        assert_eq!(f.handle.finish(last).await.unwrap(), first);
        tokio::time::advance(
            Duration::from_millis(u64::from(first.get()))
                .checked_sub(half)
                .unwrap()
                + Duration::from_millis(1),
        )
        .await;
        failed(&f.handle, DictationReason::TimedOut).await;
        f.task.await.unwrap();
    })
    .await
    .unwrap();
}
