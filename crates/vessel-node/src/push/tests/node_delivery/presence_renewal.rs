//! A paired device's presence while the store cannot write: the report is
//! still answered, and its lease renewal was tried beside it (the node
//! reviewer's R1 on TRACKER 815; `push/renewal.rs`).
use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn presence_is_answered_when_its_lease_renewal_fails() {
    Box::pin(vessel_testkit::within(
        std::time::Duration::from_secs(60),
        async {
            let receiver = Receiver::start(StatusCode::CREATED).await;
            let fixture = Fixture::start(receiver.url.clone()).await;
            let (id, device) = fixture.pair().await;
            let push = std::sync::Arc::clone(fixture.node.secure.service.0.push());
            // Every write from here fails with `Closed`, as while the node
            // stops. No subscription was made, so the renewal is due at once.
            push.store().close().await;
            for activity in [PushActivity::Active, PushActivity::Unchanged] {
                device
                    .report_push_presence(ReportPushPresence {
                        idle_age: None,
                        notifies: true,
                        focus: PushFocus::Background,
                        activity,
                    })
                    .await
                    .unwrap();
            }
            assert!(
                !push.renewals.due(id, chrono::Utc::now()),
                "the first report tried the renewal, so it is not due again this hour"
            );
            drop(device);
            fixture.close().await;
            receiver.close().await;
        },
    ))
    .await
    .unwrap();
}
