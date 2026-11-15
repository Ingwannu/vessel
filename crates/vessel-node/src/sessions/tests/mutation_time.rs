//! Actual writer acknowledgements reach live and ended runtime metadata.

use super::{Fixture, ready};
use std::time::Duration;
use vessel_proto::{Creator, EventSeq, Mark, NodeId, SessionId};
use vessel_session::LaunchOptions;

const MUTATION_PUBLICATION_BUDGET: Duration = Duration::from_secs(30);

#[tokio::test]
async fn creation_append_mark_and_handover_keep_acknowledged_clock_data() {
    vessel_testkit::within(
        MUTATION_PUBLICATION_BUDGET,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let created = fixture
                .sessions
                .create(fixture.structured(), LaunchOptions::default(), None)
                .await
                .unwrap();
            let initial = created.updated_at.unwrap();
            let handle = fixture.runtime.get(created.id).unwrap();
            let _running = ready(&handle).await.unwrap();
            let _page = fixture
                .sessions
                .read(created.id, EventSeq::ZERO, 100)
                .await
                .unwrap();
            let before = handle.info().updated_at.unwrap();
            assert!(before >= initial);
            let marked = fixture.sessions.mark(created.id, Mark::Pin).await.unwrap();
            assert!(marked.marks.pinned);
            assert!(marked.updated_at.unwrap() > before);
            assert!(
                fixture
                    .store
                    .session(created.id)
                    .await
                    .unwrap()
                    .unwrap()
                    .updated_at
                    >= marked.updated_at
            );
            assert_eq!(
                handle.watch_info().borrow().updated_at,
                None,
                "the raw actor watch must not present a stale durable clock"
            );
            handle.kill();
            let _ended = handle.ended().await;
            fixture.sessions.wait_stored(created.id).await.unwrap();
            let stored = fixture.store.session(created.id).await.unwrap().unwrap();
            assert_eq!(handle.info().updated_at, stored.updated_at);
            assert!(stored.updated_at > marked.updated_at);
            fixture.stop().await;
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn live_detach_publishes_its_ack_and_repeating_it_does_not_erase_time() {
    vessel_testkit::within(
        MUTATION_PUBLICATION_BUDGET,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let created = fixture
                .sessions
                .create(
                    fixture.structured(),
                    LaunchOptions {
                        created_by: Creator::Agent {
                            node: NodeId::random(),
                            session: SessionId::random(),
                        },
                        ..LaunchOptions::default()
                    },
                    None,
                )
                .await
                .unwrap();
            let handle = fixture.runtime.get(created.id).unwrap();
            let _running = ready(&handle).await.unwrap();
            let detached = fixture.sessions.detach(created.id).await.unwrap();
            assert!(detached.session.updated_at.unwrap() > created.updated_at.unwrap());
            assert_eq!(handle.info().detach_receipt, Some(detached.receipt));
            assert!(handle.info().updated_at >= detached.session.updated_at);
            let repeated = fixture.sessions.detach(created.id).await.unwrap();
            assert_eq!(repeated.receipt, detached.receipt);
            assert!(repeated.session.updated_at >= detached.session.updated_at);
            fixture.runtime.shutdown().await;
            fixture.settle().await;
            fixture.stop().await;
        }),
    )
    .await
    .unwrap();
}
