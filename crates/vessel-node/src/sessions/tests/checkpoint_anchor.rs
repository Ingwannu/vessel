//! The first hook waits for both node registration and the actual shared writer.
use super::*;
use vessel_proto::EventSeq;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_initial_anchor_waits_for_registration_and_actual_persistence() {
    vessel_testkit::within(std::time::Duration::from_secs(60), async {
        let fixture = Fixture::new().await;
        let created = fixture.runtime.create(fixture.structured()).await.unwrap();
        let handle = created.handle;
        let info = {
            let waiting = fixture.sessions.checkpoint_anchor(
                handle.id(),
                EventSeq::ZERO,
                tokio::time::Instant::now() + std::time::Duration::from_secs(30),
            );
            tokio::pin!(waiting);
            assert!(
                futures_util::poll!(waiting.as_mut()).is_pending(),
                "runtime visibility alone is not node registration"
            );
            let durability = fixture.sessions.track(&handle, EventSeq::ZERO);
            assert!(
                futures_util::poll!(waiting.as_mut()).is_pending(),
                "registration alone is not durable anchor publication"
            );
            fixture.store.create_session(handle.info()).await.unwrap();
            fixture
                .sessions
                .persist(handle.clone(), created.durable, durability);
            waiting.await.unwrap()
        };
        assert_eq!(info.id, handle.id());
        let page = fixture
            .store
            .read_events(handle.id(), EventSeq::ZERO, 1)
            .await
            .unwrap();
        assert_eq!(page.records.len(), 1);
        handle.kill();
        handle.ended().await;
        fixture.settle().await;
        fixture.stop().await;
    })
    .await
    .unwrap();
}
