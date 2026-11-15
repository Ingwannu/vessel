//! Full output buffers never gate stop selection or an active finish deadline.
use super::*;

async fn saturated(f: &mut Fixture) -> (FakeRequest, FakeRequest) {
    feed(&f.handle, 0, COMMIT_SECONDS, 1000).await;
    let first = f.request().await;
    feed(&f.handle, COMMIT_SECONDS, COMMIT_SECONDS, 1000).await;
    let second = f.request().await;
    first.reply.send(Ok(text("first"))).unwrap();
    // The next provider admission proves the prior completion was folded.
    feed(&f.handle, COMMIT_SECONDS * 2, COMMIT_SECONDS, 1000).await;
    let third = f.request().await;
    second.reply.send(Ok(text("second"))).unwrap();
    feed(&f.handle, COMMIT_SECONDS * 3, COMMIT_SECONDS, 1000).await;
    let fourth = f.request().await;
    (third, fourth)
}

#[tokio::test]
async fn cancel_is_not_queued_behind_a_full_partial_buffer_or_provider_jobs() {
    tokio::time::timeout(TEST_LIMIT, async {
        let mut f = Fixture::new();
        let (mut third, mut fourth) = saturated(&mut f).await;
        f.handle.cancel().await;
        third.reply.closed().await;
        fourth.reply.closed().await;
        assert_eq!(
            f.handle.control.chosen().unwrap().reason(),
            Some(DictationReason::Cancelled)
        );
        f.task.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test(start_paused = true)]
async fn an_active_deadline_fires_with_full_partials_and_provider_work() {
    tokio::time::timeout(TEST_LIMIT, async {
        let mut f = Fixture::new();
        let (third, mut fourth) = saturated(&mut f).await;
        let last = ChunkSeq::new(u32::try_from(COMMIT_SECONDS * 4).unwrap());
        let within = f.handle.finish(last).await.unwrap();
        // Its success may now wait for partial capacity; it cannot disarm time.
        third.reply.send(Ok(text("third"))).unwrap();
        tokio::time::advance(
            Duration::from_millis(u64::from(within.get())) + Duration::from_nanos(1),
        )
        .await;
        let mut selected = f.handle.control.subscribe();
        drop(selected.wait_for(Option::is_some).await.unwrap());
        assert_eq!(
            f.handle.control.chosen().unwrap().reason(),
            Some(DictationReason::TimedOut)
        );
        fourth.reply.closed().await;
        partial(&f.handle, "first").await;
        partial(&f.handle, "first second").await;
        failed(&f.handle, DictationReason::TimedOut).await;
        f.task.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cancel_settles_a_next_already_queued_before_actor_admission() {
    tokio::time::timeout(TEST_LIMIT, async {
        let f = Fixture::new();
        let (reply, response) = oneshot::channel();
        f.handle.next.send(reply).await.unwrap();
        // No intervening await gives the owner a chance to admit the waiter.
        let _chosen = f.handle.control.stop(
            Arc::new(StreamError::Cancelled),
            crate::dictation::control::Retention::Forget,
        );
        match response.await.unwrap() {
            Reply::Update(Event::End(End::Failed(error))) => {
                assert_eq!(error.reason(), Some(DictationReason::Cancelled));
            }
            Reply::Update(Event::Partial(_) | Event::End(End::Final(_))) | Reply::Refused(_) => {
                panic!("queued next lost its terminal")
            }
        }
        f.task.await.unwrap();
    })
    .await
    .unwrap();
}
