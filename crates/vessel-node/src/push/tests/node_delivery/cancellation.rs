//! Deterministic preparation barriers, with completion observed before shutdown.
use super::*;

#[derive(Clone, Copy)]
enum DuringPreparation {
    Read,
    Revoke,
    ShrinkScope,
    DeleteDevice,
    Renew,
    Focus,
}

async fn cancels_before_admission(stage: crate::push::PreparedStage, action: DuringPreparation) {
    let mut receiver = Receiver::start(StatusCode::CREATED).await;
    let (ready, mut paused) = mpsc::channel(1);
    let (finished, mut completed) = mpsc::channel(1);
    let hook = std::sync::Arc::new(crate::push::Preparation {
        stages: vec![stage],
        ready,
        finished,
    });
    let fixture = Fixture::start_with(receiver.url.clone(), |config| {
        config.push_preparation = Some(hook);
    })
    .await;
    let (id, device) = fixture.pair().await;
    let (_, subscription) = subscriber();
    let _lease = device.subscribe_push(subscription.clone()).await.unwrap();
    let (session, _stream) = fixture.asking().await;
    let release = step("push preparation barrier", paused.recv())
        .await
        .unwrap();
    match action {
        DuringPreparation::Read => {
            let page = fixture.owner.inbox(ListInbox::default()).await.unwrap();
            fixture
                .owner
                .mark_inbox(page.entries[0].key, InboxRead::Read)
                .await
                .unwrap();
        }
        DuringPreparation::Revoke => fixture.owner.revoke_device(id).await.unwrap(),
        DuringPreparation::ShrinkScope => {
            let db =
                rusqlite::Connection::open(fixture.temp.path().join("data/vessel.db")).unwrap();
            let changed = db
                .execute(
                    "UPDATE devices SET scopes='[\"operate\"]' WHERE id=?1",
                    [id.to_string()],
                )
                .unwrap();
            assert_eq!(changed, 1);
        }
        DuringPreparation::DeleteDevice => {
            let db =
                rusqlite::Connection::open(fixture.temp.path().join("data/vessel.db")).unwrap();
            let changed = db
                .execute("DELETE FROM devices WHERE id=?1", [id.to_string()])
                .unwrap();
            assert_eq!(changed, 1);
        }
        DuringPreparation::Renew => {
            let _lease = device.subscribe_push(subscription).await.unwrap();
        }
        DuringPreparation::Focus => fixture
            .owner
            .report_push_presence(ReportPushPresence {
                idle_age: None,
                focus: PushFocus::Session { session },
                activity: PushActivity::Active,
            })
            .await
            .unwrap(),
    }
    release.send(()).unwrap();
    // Observe the completed attempt before shutdown, so cancellation by
    // shutdown cannot make a missing admission check pass this assertion.
    assert!(
        step("prepared push attempt completed", completed.recv())
            .await
            .unwrap()
            .is_none()
    );
    assert!(receiver.inbox.try_recv().is_err());
    fixture.close().await;
    receiver.close().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn state_changes_during_crypto_or_dns_preparation_cancel_before_http_admission() {
    Box::pin(vessel_testkit::within(
        std::time::Duration::from_secs(60),
        async {
            for stage in [
                crate::push::PreparedStage::Crypto,
                crate::push::PreparedStage::Transport,
            ] {
                for action in [
                    DuringPreparation::Read,
                    DuringPreparation::Revoke,
                    DuringPreparation::ShrinkScope,
                    DuringPreparation::DeleteDevice,
                    DuringPreparation::Renew,
                    DuringPreparation::Focus,
                ] {
                    cancels_before_admission(stage, action).await;
                }
            }
        },
    ))
    .await
    .unwrap();
}

#[derive(Clone, Copy)]
enum FirstGone {
    Read,
    Withdraw,
}
impl FirstGone {
    async fn apply(
        self,
        owner: &NodeClient,
        first_key: vessel_proto::inbox::InboxKey,
        first_stream: &mut vessel_client::SessionStream,
    ) {
        match self {
            Self::Read => owner.mark_inbox(first_key, InboxRead::Read).await.unwrap(),
            Self::Withdraw => {
                owner
                    .send(first_key.session, SessionInput::CancelTurn { turn: None })
                    .await
                    .unwrap();
                loop {
                    let record = step("first request withdrawn", first_stream.next())
                        .await
                        .unwrap()
                        .unwrap();
                    if matches!(
                        record.event,
                        SessionEvent::Agent {
                            event: AgentEvent::PermissionResolved { .. }
                                | AgentEvent::TurnEnded { .. }
                        }
                    ) {
                        break;
                    }
                }
                let _committed = step(
                    "withdrawal committed before admission",
                    owner.read(first_key.session, EventSeq::ZERO, 1),
                )
                .await
                .unwrap();
                assert!(owner.inbox_entry(first_key).await.unwrap().is_none());
            }
        }
    }
}
async fn still_sends_second(action: FirstGone) {
    let mut receiver = Receiver::start(StatusCode::CREATED).await;
    let (ready, mut paused) = mpsc::channel(2);
    let (finished, mut completed) = mpsc::channel(2);
    let hook = std::sync::Arc::new(crate::push::Preparation {
        stages: vec![
            crate::push::PreparedStage::Observed,
            crate::push::PreparedStage::Transport,
        ],
        ready,
        finished,
    });
    let fixture = Fixture::start_with(receiver.url.clone(), |config| {
        config.push_preparation = Some(hook);
    })
    .await;
    let (_, device) = fixture.pair().await;
    let (private, subscription) = subscriber();
    let _lease = device.subscribe_push(subscription.clone()).await.unwrap();
    let (first, mut first_stream) = fixture.asking().await;
    let observed = step("push preparation barrier", paused.recv())
        .await
        .unwrap();
    // Both sources are committed before the worker captures its burst head.
    let (second, _second_stream) = fixture.asking().await;
    observed.send(()).unwrap();
    let transport = step("push preparation barrier", paused.recv())
        .await
        .unwrap();
    let page = fixture.owner.inbox(ListInbox::default()).await.unwrap();
    let first_key = page
        .entries
        .iter()
        .find(|entry| entry.key.session == first)
        .unwrap()
        .key;
    let second_key = page
        .entries
        .iter()
        .find(|entry| entry.key.session == second)
        .unwrap()
        .key;
    action
        .apply(&fixture.owner, first_key, &mut first_stream)
        .await;
    transport.send(()).unwrap();
    let admitted = step("prepared push attempt completed", completed.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        match admitted {
            vessel_proto::notifications::NotificationSource::Session { key, .. } => key,
            vessel_proto::notifications::NotificationSource::Terminal { .. } =>
                panic!("expected session admission"),
        },
        second_key,
        "first attempt must admit B, not a later outcome of A"
    );
    let message = step("second source HTTP request arrived", receiver.inbox.recv())
        .await
        .unwrap();
    assert_eq!(
        web_push_native::decrypt(
            message.body.to_vec(),
            &private,
            &web_push_native::Auth::from(*subscription.auth.expose())
        )
        .unwrap(),
        crate::push::crypto::PAYLOAD
    );
    fixture.close().await;
    receiver.close().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reading_or_withdrawing_first_source_during_preparation_does_not_consume_second_source() {
    Box::pin(vessel_testkit::within(
        std::time::Duration::from_secs(60),
        async {
            for action in [FirstGone::Read, FirstGone::Withdraw] {
                Box::pin(still_sends_second(action)).await;
            }
        },
    ))
    .await
    .unwrap();
}
