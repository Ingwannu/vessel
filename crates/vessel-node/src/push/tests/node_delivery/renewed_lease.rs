//! A late HTTP response belongs to the lease admitted before that request.
use super::*;

async fn gone_after_admission(status: StatusCode, renew: bool) {
    let mut receiver = Receiver::delayed(status).await;
    let (ready, _paused) = mpsc::channel(1);
    let (finished, mut completed) = mpsc::channel(1);
    let hook = std::sync::Arc::new(crate::push::Preparation {
        stages: Vec::new(),
        ready,
        finished,
    });
    let fixture = Fixture::start_with(receiver.url.clone(), |config| {
        config.push_preparation = Some(hook);
    })
    .await;
    let (id, device) = fixture.pair().await;
    let (_, subscription) = subscriber();
    let old = device.subscribe_push(subscription.clone()).await.unwrap();
    let (_session, _stream) = fixture.asking().await;
    // Receiving HTTP proves final admission already happened. The server's
    // response is held until the same-capability renewal has committed.
    let packet = step("admitted push reaches HTTP receiver", receiver.inbox.recv())
        .await
        .unwrap();
    let expected = if renew {
        let renewed = device.subscribe_push(subscription).await.unwrap();
        assert!(renewed.expires_at > old.expires_at);
        Some(renewed.expires_at)
    } else {
        None
    };
    packet.reply.unwrap().send(()).unwrap();
    assert!(
        step("gone response cleanup commits", completed.recv())
            .await
            .unwrap()
            .is_some()
    );
    let db = rusqlite::Connection::open(fixture.temp.path().join("data/vessel.db")).unwrap();
    let retained: Option<i64> = {
        use rusqlite::OptionalExtension as _;
        db.query_row(
            "SELECT expires_at FROM push_subscriptions WHERE device=?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .optional()
        .unwrap()
    };
    assert_eq!(
        retained,
        expected.map(|expiry| expiry.timestamp_nanos_opt().unwrap())
    );
    drop(db);
    fixture.close().await;
    receiver.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_404_and_410_keep_same_subscription_renewals_but_remove_the_attempted_lease() {
    Box::pin(vessel_testkit::within(
        std::time::Duration::from_secs(60),
        async {
            for status in [StatusCode::NOT_FOUND, StatusCode::GONE] {
                for renew in [false, true] {
                    gone_after_admission(status, renew).await;
                }
            }
        },
    ))
    .await
    .unwrap();
}
