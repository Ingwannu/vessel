//! A fresh worker's watermark excludes persisted unread outcomes, while a
//! retained lease still receives new attention using the same VAPID identity.
use super::*;

impl Fixture {
    async fn restart(
        self,
        origin: url::Url,
        hook: std::sync::Arc<crate::push::Preparation>,
    ) -> Self {
        self.node.stop().await.unwrap();
        let mut config = Self::config(self.temp.path(), origin);
        config.push_preparation = Some(hook);
        let node = Node::start(config).await.unwrap();
        let url: NodeUrl = format!("ws://{}", node.address().unwrap()).parse().unwrap();
        let token: BearerToken = DataDir::new(self.temp.path().join("data"))
            .read_token()
            .unwrap()
            .expose()
            .parse()
            .unwrap();
        let owner = NodeClient::connect(&url, &token, client_info())
            .await
            .unwrap();
        Self {
            temp: self.temp,
            node,
            owner,
        }
    }
}
async fn finish(
    fixture: &Fixture,
    session: vessel_proto::SessionId,
    stream: &mut vessel_client::SessionStream,
) {
    let page = fixture.owner.inbox(ListInbox::default()).await.unwrap();
    let entry = page
        .entries
        .iter()
        .find(|entry| entry.key.session == session)
        .unwrap();
    let vessel_proto::inbox::InboxItem::Permission { request, .. } = &entry.item else {
        panic!("the fake permission must be live");
    };
    fixture
        .owner
        .send(
            session,
            SessionInput::AnswerPermission {
                id: request.id,
                outcome: vessel_proto::PermissionOutcome::Selected {
                    option: vessel_proto::PermissionOptionId::new("allow-once"),
                },
            },
        )
        .await
        .unwrap();
    loop {
        let record = step("old turn finishes before restart", stream.next())
            .await
            .unwrap()
            .unwrap();
        if matches!(
            record.event,
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded { .. }
            }
        ) {
            break;
        }
    }
    let _committed = fixture
        .owner
        .read(session, EventSeq::ZERO, 1)
        .await
        .unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restart_keeps_the_key_and_subscription_but_does_not_replay_unread_outcomes() {
    Box::pin(vessel_testkit::within(
        std::time::Duration::from_secs(60),
        async {
            let mut receiver = Receiver::start(StatusCode::CREATED).await;
            let fixture = Fixture::start(receiver.url.clone()).await;
            let (device_id, device) = fixture.pair().await;
            let key = device.push_key().await.unwrap();
            let (private, subscription) = subscriber();
            let _lease = device.subscribe_push(subscription.clone()).await.unwrap();
            drop(device);
            let (old_session, mut stream) = fixture.asking().await;
            finish(&fixture, old_session, &mut stream).await;
            let old = fixture
                .owner
                .inbox(ListInbox::default())
                .await
                .unwrap()
                .entries
                .remove(0);
            assert_eq!(old.item, vessel_proto::inbox::InboxItem::Finished);
            assert_eq!(old.read, InboxRead::Unread);
            let (ready, mut paused) = mpsc::channel(2);
            let (finished, mut completed) = mpsc::channel(2);
            let hook = std::sync::Arc::new(crate::push::Preparation {
                stages: vec![crate::push::PreparedStage::Observed],
                ready,
                finished,
            });
            let fixture = fixture.restart(receiver.url.clone(), hook).await;
            // Shutdown joined the prior worker; no earlier HTTP attempt remains.
            while receiver.inbox.try_recv().is_ok() {}
            let (_, fresh_device) = fixture.pair().await;
            assert_eq!(fresh_device.push_key().await.unwrap(), key);
            drop(fresh_device);
            let (new_session, _stream) = fixture.asking().await;
            let release = step("new attention wakes fresh worker", paused.recv())
                .await
                .unwrap();
            release.send(()).unwrap();
            let admitted = step("fresh worker finishes its attempt", completed.recv())
                .await
                .unwrap()
                .unwrap();
            let vessel_proto::notifications::NotificationSource::Session { key, .. } = admitted
            else {
                panic!("expected session admission");
            };
            assert_eq!(
                key.session, new_session,
                "persisted unread attention must not replay"
            );
            assert_ne!(key, old.key);
            let sent = step(
                "retained subscription receives new attention",
                receiver.inbox.recv(),
            )
            .await
            .unwrap();
            assert_eq!(
                web_push_native::decrypt(
                    sent.body.to_vec(),
                    &private,
                    &web_push_native::Auth::from(*subscription.auth.expose())
                )
                .unwrap(),
                crate::push::crypto::PAYLOAD
            );
            let retained = fixture.owner.inbox_entry(old.key).await.unwrap().unwrap();
            assert_eq!(retained.read, InboxRead::Unread);
            let db =
                rusqlite::Connection::open(fixture.temp.path().join("data/vessel.db")).unwrap();
            let subscriptions: i64 = db
                .query_row(
                    "SELECT count(*) FROM push_subscriptions WHERE device=?1",
                    [device_id.to_string()],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(subscriptions, 1);
            drop(db);
            fixture.close().await;
            receiver.close().await;
        },
    ))
    .await
    .unwrap();
}
