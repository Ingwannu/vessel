//! Real detach effects use the checked invalidation producer, not capture APIs.

use super::*;
use crate::agents::ExecutionTestWatch;
use crate::principal::Principal;
use futures_util::FutureExt as _;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use vessel_proto::{EventRecord, EventSeq, timeline::ExecutionInvalidation};
use vessel_session::SessionHandle;
use vessel_store::StoreError;

const TEST_BUDGET: Duration = Duration::from_secs(40);
const SCENARIO_BUDGET: Duration = Duration::from_secs(25);
const PHASE_BUDGET: Duration = Duration::from_secs(5);

struct Controlled {
    handle: SessionHandle,
    durable: mpsc::Receiver<Arc<EventRecord>>,
    _durability: watch::Sender<crate::persist::Durability>,
}

async fn controlled(
    fixture: &Fixture,
    controls: &mut Vec<Controlled>,
    creator: Creator,
) -> SessionHandle {
    let created = tokio::time::timeout(
        PHASE_BUDGET,
        fixture.runtime.create_with(
            fixture.spec(),
            LaunchOptions {
                created_by: creator,
                ..LaunchOptions::default()
            },
        ),
    )
    .await
    .expect("controlled fake launch phase")
    .unwrap();
    let id = created.handle.id();
    let _credential = fixture
        .sessions
        .agents
        .reserve(id, &Principal::Owner, false)
        .unwrap();
    let durability = fixture.sessions.track(&created.handle, EventSeq::ZERO);
    let handle = created.handle.clone();
    // Retain the actual child before another fallible phase, so the outer
    // owner can shut it down even if persistence or an assertion fails.
    controls.push(Controlled {
        handle: created.handle,
        durable: created.durable,
        _durability: durability,
    });
    tokio::time::timeout(PHASE_BUDGET, fixture.store.create_session(handle.info()))
        .await
        .expect("initial durable metadata phase")
        .unwrap();
    handle
}

fn observe(fixture: &Fixture, id: SessionId) -> ExecutionTestWatch {
    fixture
        .sessions
        .agents
        .watch_execution_for_test(id)
        .unwrap()
}

fn creator() -> Creator {
    Creator::Agent {
        node: NodeId::random(),
        session: SessionId::random(),
    }
}

async fn stop(fixture: Fixture, controls: Vec<Controlled>) {
    // The test owns record consumption rather than an unobserved persistence task.
    for control in controls {
        control.handle.kill();
        drop(control.durable);
    }
    tokio::time::timeout(PHASE_BUDGET, fixture.stop())
        .await
        .expect("owned fake shutdown phase");
}

type Scenario<'a> = Pin<Box<dyn Future<Output = ()> + 'a>>;

async fn exercise(
    scenario: impl for<'a> FnOnce(&'a Fixture, &'a mut Vec<Controlled>) -> Scenario<'a>,
) {
    vessel_testkit::within(TEST_BUDGET, async {
        let fixture = tokio::time::timeout(PHASE_BUDGET, Fixture::new())
            .await
            .expect("fixture setup phase");
        let mut controls = Vec::new();
        // Assertion panics and a stuck phase both return to the same owner.
        // Cleanup is joined before preserving the original failure.
        let result = AssertUnwindSafe(tokio::time::timeout(
            SCENARIO_BUDGET,
            scenario(&fixture, &mut controls),
        ))
        .catch_unwind()
        .await;
        stop(fixture, controls).await;
        match result {
            Ok(Ok(())) => {}
            Ok(Err(elapsed)) => panic!("detach scenario phase: {elapsed}"),
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
    .await
    .expect("complete owned detach execution regression");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn first_detach_rotates_once_but_repeat_and_root_refusal_do_not() {
    exercise(|fixture, controls| Box::pin(async move {
        let child = controlled(fixture, controls, creator()).await;
        let root = controlled(fixture, controls, Creator::Owner).await;
        let id = child.id();
        let mut watched = observe(fixture, id);
        let root_watch = observe(fixture, root.id());
        let before = watched.current();
        let root_before = root_watch.current();
        let pid = child.pid().expect("the owned fake is running");
        let detached = tokio::time::timeout(PHASE_BUDGET, fixture.sessions.detach(id))
            .await
            .expect("first commit and publication phase")
            .unwrap();
        assert!(watched.has_changed().unwrap());
        assert_ne!(watched.current().0, before.0);
        assert_eq!(watched.current().1, Some(ExecutionInvalidation::Detach));
        assert_eq!(child.pid(), Some(pid));
        assert!(!child.has_ended());
        assert_eq!(child.info().detach_receipt, Some(detached.receipt));
        watched.mark_seen();
        let after = watched.current();
        let repeat = tokio::time::timeout(PHASE_BUDGET, fixture.sessions.detach(id))
            .await
            .expect("repeat publication phase")
            .unwrap();
        assert_eq!(repeat.receipt, detached.receipt);
        assert_eq!(repeat.disposition, DetachDisposition::AlreadyDetached);
        assert!(!watched.has_changed().unwrap());
        assert_eq!(watched.current(), after);
        let refused = tokio::time::timeout(
            PHASE_BUDGET,
            fixture.sessions.detach(root.id()),
        )
        .await
        .expect("root validation phase");
        assert!(matches!(refused, Err(ServiceError::Store(StoreError::NotChild(found))) if found == root.id()));
        assert!(!root_watch.has_changed().unwrap());
        assert_eq!(root_watch.current(), root_before);
        // No registry entry is minted to represent a missing execution.
        fixture.sessions.agents.forget(id);
        assert!(fixture.sessions.agents.watch_execution_for_test(id).is_none());
        let repaired = tokio::time::timeout(PHASE_BUDGET, fixture.sessions.detach(id))
            .await
            .expect("stored repeat without an execution phase")
            .unwrap();
        assert_eq!(repaired.receipt, detached.receipt);
        assert!(fixture.sessions.agents.watch_execution_for_test(id).is_none());
    }))
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn held_transition_invalidates_before_a_failing_first_store_effect() {
    exercise(|fixture, controls| {
        Box::pin(async move {
            let child = controlled(fixture, controls, creator()).await;
            let id = child.id();
            let watched = observe(fixture, id);
            let before = watched.current();
            tokio::time::timeout(PHASE_BUDGET, fixture.store.close())
                .await
                .expect("writer close phase");
            // Reads remain available. Only the first detach write must fail, making
            // an invalidation incorrectly placed after that write observable.
            assert!(fixture.store.session(id).await.unwrap().is_some());
            let marking = fixture.sessions.marking.lock().await;
            let mut request = Box::pin(fixture.sessions.detach(id));
            assert!(futures_util::poll!(request.as_mut()).is_pending());
            assert_eq!(fixture.tasks.len(), 1);
            assert!(!watched.has_changed().unwrap());
            assert_eq!(watched.current(), before);
            drop(marking);
            let result = tokio::time::timeout(PHASE_BUDGET, request)
                .await
                .expect("validated transition and failed first write phase");
            assert!(matches!(
                result,
                Err(ServiceError::Store(StoreError::Closed))
            ));
            assert!(watched.has_changed().unwrap());
            assert_ne!(watched.current().0, before.0);
            assert_eq!(watched.current().1, Some(ExecutionInvalidation::Detach));
            assert_eq!(
                fixture
                    .store
                    .session(id)
                    .await
                    .unwrap()
                    .unwrap()
                    .detach_receipt,
                None
            );
            assert_eq!(child.info().detach_receipt, None);
            assert!(!child.has_ended());
        })
    })
    .await;
}
