//! Force a metadata publication before creating its empty durable row.

use super::Fixture;
use crate::persist::{self, Persisted};
use std::time::Duration;
use vessel_proto::{EventSeq, SessionTitle};

const CREATION_BOUNDARY_BUDGET: Duration = Duration::from_secs(30);

#[tokio::test]
async fn an_early_rename_enters_the_row_only_when_its_record_is_appended() {
    vessel_testkit::within(
        CREATION_BOUNDARY_BUDGET,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let original: SessionTitle = "Creation title".parse().unwrap();
            let changed: SessionTitle = "Ahead of persistence".parse().unwrap();
            let created = fixture
                .runtime
                .create(fixture.structured().with_title(original.clone()))
                .await
                .unwrap();
            let handle = created.handle;
            let id = handle.id();
            let current = handle.rename(changed.clone()).await.unwrap();
            assert_eq!(current.title, Some(changed.clone()));
            assert!(current.next_seq > EventSeq::ZERO);
            // The receiver remains held until after this check. No sleep or
            // scheduler race can let this rename be durable already.
            fixture
                .store
                .create_session(*created.initial)
                .await
                .unwrap();
            let stored = fixture.store.session(id).await.unwrap().unwrap();
            assert_eq!(stored.title, Some(original));
            assert_eq!(stored.next_seq, EventSeq::ZERO);
            assert_eq!(stored.spec, current.spec);
            let durability = fixture.sessions.track(&handle, EventSeq::ZERO);
            handle.kill();
            let _ended = handle.ended().await;
            let persisted = persist::run(
                fixture.store.clone(),
                handle.clone(),
                created.durable,
                durability,
            )
            .await;
            assert_eq!(persisted, Persisted::Complete);
            let stored = fixture.store.session(id).await.unwrap().unwrap();
            assert_eq!(stored.title, Some(changed));
            assert_eq!(stored.next_seq, handle.info().next_seq);
            fixture.sessions.hand_over(handle, persisted).await;
            fixture.stop().await;
        }),
    )
    .await
    .unwrap();
}
