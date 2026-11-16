//! Exercise the actual read ingress and shared metadata lease, without wire
//! assignment or a new test-only operation lane.
use super::*;
use std::time::Duration;
use vessel_proto::{EventSeq, Mark};

const ABANDONED_READS: usize = 16;
const SETTLEMENT_DEADLINE: Duration = Duration::from_secs(5);
const HELD_STORE_OBSERVATION: Duration = Duration::from_millis(20);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_unadmitted_reads_never_create_retained_tasks_or_effects() {
    let fixture = Fixture::new().await;
    let info = SessionInfo::new(
        SessionId::random(),
        fixture.structured(),
        chrono::Utc::now(),
    );
    let id = info.id;
    fixture.store.create_session(info).await.unwrap();
    fixture
        .store
        .mark(id, Mark::Unread, chrono::Utc::now())
        .await
        .unwrap();
    assert_eq!(fixture.tasks.len(), 0);
    let order = fixture.sessions.marking.lock().await;
    for _ in 0..ABANDONED_READS {
        let mut request = Box::pin(fixture.sessions.read_attention(id, EventSeq::ZERO));
        assert!(futures_util::poll!(request.as_mut()).is_pending());
        drop(request);
        assert_eq!(fixture.tasks.len(), 0, "waiting RPCs have no accepted task");
    }
    assert!(
        fixture
            .store
            .session(id)
            .await
            .unwrap()
            .unwrap()
            .marks
            .unread
    );
    drop(order);
    fixture.settle().await;
    assert_eq!(fixture.tasks.len(), 0);
    assert!(
        fixture
            .store
            .session(id)
            .await
            .unwrap()
            .unwrap()
            .marks
            .unread
    );
    fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accepted_read_keeps_the_same_lease_through_store_and_live_publication_after_drop() {
    let fixture = Fixture::new().await;
    // The real fake ACP runs, but persistence remains explicitly controlled so
    // TaskTracker counts only this actual read rather than a durability worker.
    let vessel_session::Created { handle, durable } = fixture
        .runtime
        .create_with(fixture.structured(), LaunchOptions::default())
        .await
        .unwrap();
    let _ready = ready(&handle).await.unwrap();
    let id = handle.id();
    let _durability = fixture.sessions.track(&handle, EventSeq::ZERO);
    fixture.store.create_session(handle.info()).await.unwrap();
    let kept = fixture
        .store
        .mark(id, Mark::Unread, chrono::Utc::now())
        .await
        .unwrap();
    handle.set_marks(kept);
    assert!(handle.info().marks.unread);
    assert_eq!(fixture.tasks.len(), 0);

    // A real SQLite writer reservation prevents the accepted operation from
    // committing. No substitute Store or synthetic ACK stands in for it.
    let connection =
        rusqlite::Connection::open(DataDir::new(fixture.data.path()).database_path()).unwrap();
    connection.execute_batch("BEGIN IMMEDIATE").unwrap();
    {
        let mut request = Box::pin(fixture.sessions.read_attention(id, EventSeq::ZERO));
        assert!(futures_util::poll!(request.as_mut()).is_pending());
        assert_eq!(fixture.tasks.len(), 1);
    }
    assert!(fixture.sessions.marking.try_lock().is_err());
    let _first_close = fixture.tasks.close();
    assert!(
        tokio::time::timeout(HELD_STORE_OBSERVATION, fixture.tasks.wait())
            .await
            .is_err()
    );
    assert_eq!(fixture.tasks.len(), 1);
    assert!(fixture.sessions.marking.try_lock().is_err());
    assert!(handle.info().marks.unread);
    assert!(
        fixture
            .store
            .session(id)
            .await
            .unwrap()
            .unwrap()
            .marks
            .unread
    );
    connection.execute_batch("ROLLBACK").unwrap();
    drop(connection);
    tokio::time::timeout(SETTLEMENT_DEADLINE, fixture.tasks.wait())
        .await
        .expect("the accepted read owns durable and live settlement after its caller leaves");
    assert_eq!(fixture.tasks.len(), 0);
    assert!(fixture.sessions.marking.try_lock().is_ok());
    assert!(
        !fixture
            .store
            .session(id)
            .await
            .unwrap()
            .unwrap()
            .marks
            .unread
    );
    assert!(!handle.info().marks.unread);
    drop(durable);
    fixture.stop().await;
}
