//! Real fake-backed sessions exercise the same authorization/publication path.

mod execution;

use super::*;
use vessel_proto::{
    Creator, NodeId, SessionRef,
    subagents::{DetachDisposition, Parentage},
};

async fn child(fixture: &Fixture, parent: SessionRef) -> SessionInfo {
    fixture
        .sessions
        .create(
            fixture.spec(),
            LaunchOptions {
                created_by: Creator::Agent {
                    node: parent.node,
                    session: parent.session,
                },
                ..LaunchOptions::default()
            },
            None,
        )
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detach_preserves_the_process_and_breaks_live_and_stored_ancestor_authority() {
    let fixture = Fixture::new().await;
    let here = NodeId::random();
    let ancestor = SessionRef {
        node: NodeId::random(),
        session: SessionId::random(),
    };
    let parent = child(&fixture, ancestor).await;
    let parent_ref = SessionRef {
        node: here,
        session: parent.id,
    };
    let descendant = child(&fixture, parent_ref).await;
    assert!(
        fixture
            .sessions
            .check_parentage(here, ancestor, descendant.id)
            .await
            .is_ok()
    );
    let handle = fixture.sessions.live(parent.id).unwrap();
    let pid = handle.pid().expect("the fake session runs before detach");
    let detached = fixture.sessions.detach(parent.id).await.unwrap();
    assert_eq!(detached.disposition, DetachDisposition::Detached);
    assert_eq!(handle.pid(), Some(pid));
    assert!(!handle.has_ended());
    assert_eq!(detached.session.id, parent.id);
    assert_eq!(detached.session.spec, parent.spec);
    assert_eq!(detached.session.created_by, parent.created_by);
    assert_eq!(handle.info().detach_receipt, Some(detached.receipt));
    assert!(
        matches!(fixture.sessions.check_parentage(here,ancestor,parent.id).await,Err(ServiceError::NotOwned(id)) if id==parent.id)
    );
    assert!(
        matches!(fixture.sessions.check_parentage(here,ancestor,descendant.id).await,Err(ServiceError::NotOwned(id)) if id==descendant.id)
    );
    assert!(
        fixture
            .sessions
            .check_parentage(here, parent_ref, descendant.id)
            .await
            .is_ok()
    );
    // Complete identity matters even if a different host reuses the session id.
    let wrong = SessionRef {
        node: NodeId::random(),
        session: parent_ref.session,
    };
    assert!(
        fixture
            .sessions
            .check_parentage(here, wrong, descendant.id)
            .await
            .is_err()
    );
    let repeated = fixture.sessions.detach(parent.id).await.unwrap();
    assert_eq!(repeated.receipt, detached.receipt);
    assert_eq!(repeated.disposition, DetachDisposition::AlreadyDetached);
    fixture.runtime.shutdown().await;
    fixture.settle().await;
    assert_eq!(
        fixture
            .sessions
            .get(parent.id)
            .await
            .unwrap()
            .parentage()
            .unwrap(),
        Parentage::Detached(detached.receipt)
    );
    assert!(
        fixture
            .sessions
            .check_parentage(here, ancestor, descendant.id)
            .await
            .is_err()
    );
    assert!(
        fixture
            .sessions
            .check_parentage(here, parent_ref, descendant.id)
            .await
            .is_ok()
    );
    fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn committed_receipts_override_an_older_live_actor_before_publication() {
    let fixture = Fixture::new().await;
    let here = NodeId::random();
    let ancestor = SessionRef {
        node: here,
        session: SessionId::random(),
    };
    let info = child(&fixture, ancestor).await;
    let handle = fixture.sessions.live(info.id).unwrap();
    // Reproduce the exact store-commit/live-publication window, deterministically.
    let committed = fixture
        .store
        .detach(info.id, chrono::Utc::now())
        .await
        .unwrap();
    assert_eq!(handle.info().detach_receipt, None);
    assert!(
        matches!(fixture.sessions.check_parentage(here,ancestor,info.id).await,Err(ServiceError::NotOwned(id)) if id==info.id)
    );
    let published = fixture.sessions.detach(info.id).await.unwrap();
    assert_eq!(published.receipt, committed.receipt);
    assert_eq!(handle.info().detach_receipt, Some(committed.receipt));
    fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_resume_metadata_cannot_reattach_a_detached_session() {
    let fixture = Fixture::new().await;
    let here = NodeId::random();
    let ancestor = SessionRef {
        node: here,
        session: SessionId::random(),
    };
    let info = fixture
        .sessions
        .create(
            fixture.structured(),
            LaunchOptions {
                created_by: Creator::Agent {
                    node: ancestor.node,
                    session: ancestor.session,
                },
                ..LaunchOptions::default()
            },
            None,
        )
        .await
        .unwrap();
    let handle = fixture.sessions.live(info.id).unwrap();
    let _ready = ready(&handle).await.unwrap();
    handle.kill();
    fixture.settle().await;
    let stale = fixture.sessions.ended(info.id).await.unwrap();
    assert_eq!(stale.info.detach_receipt, None);
    let detached = fixture.sessions.detach(info.id).await.unwrap();
    let claim = fixture.sessions.claim(info.id).unwrap();
    let resumed = fixture
        .sessions
        .start_again(stale, Relaunch::default(), claim)
        .await
        .unwrap();
    let _ready = ready(&resumed).await.unwrap();
    assert_eq!(resumed.info().detach_receipt, Some(detached.receipt));
    assert!(
        fixture
            .sessions
            .check_parentage(here, ancestor, info.id)
            .await
            .is_err()
    );
    fixture.stop().await;
}

/// Safety budget for the owned detach's commit/publication phase, not polling.
const DETACH_PUBLICATION_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancelled_caller_cannot_abandon_live_publication() {
    let fixture = Fixture::new().await;
    let ancestor = SessionRef {
        node: NodeId::random(),
        session: SessionId::random(),
    };
    // Keep persistence under this fixture's control so the node TaskTracker
    // contains exactly the real detach task whose RPC future is dropped.
    let vessel_session::Created { handle, durable } = fixture
        .runtime
        .create_with(
            fixture.spec(),
            LaunchOptions {
                created_by: Creator::Agent {
                    node: ancestor.node,
                    session: ancestor.session,
                },
                ..LaunchOptions::default()
            },
        )
        .await
        .unwrap();
    let id = handle.id();
    let _durability = fixture
        .sessions
        .track(&handle, vessel_proto::EventSeq::ZERO);
    fixture.store.create_session(handle.info()).await.unwrap();
    assert_eq!(fixture.tasks.len(), 0);
    let order = fixture.sessions.marking.lock().await;
    {
        let mut request = Box::pin(fixture.sessions.detach(id));
        // Starts the actual owned task, which cannot commit under this lock.
        assert!(futures_util::poll!(request.as_mut()).is_pending());
    }
    assert_eq!(fixture.tasks.len(), 1);
    let _first_close = fixture.tasks.close();
    drop(order);
    // TaskTracker's empty/closed notification proves the abandoned RPC's
    // actual node task finished; no DB reads or yields coordinate the test.
    tokio::time::timeout(DETACH_PUBLICATION_DEADLINE, fixture.tasks.wait())
        .await
        .expect("the owned detach completes commit and live publication");
    let receipt = fixture
        .store
        .session(id)
        .await
        .unwrap()
        .unwrap()
        .detach_receipt
        .unwrap();
    assert_eq!(handle.info().detach_receipt, Some(receipt));
    assert!(!handle.has_ended());
    assert_eq!(fixture.sessions.detach(id).await.unwrap().receipt, receipt);
    drop(durable);
    fixture.stop().await;
}
