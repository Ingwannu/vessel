//! Admission probes always close and join the same owner after assertion failure.
use super::*;
use crate::workspace::tests::root_case::{phase, settle};
use futures_util::FutureExt as _;
use std::panic::AssertUnwindSafe;
use vessel_workspace::physical::Directory;
const ROOT_TEST_TIMEOUT: Duration = Duration::from_secs(30);
fn scope(path: &std::path::Path) -> Arc<Ancestry> {
    std::fs::create_dir_all(path).unwrap();
    Arc::new(Directory::open(path).unwrap().ancestry().unwrap())
}
fn deadline() -> Instant {
    Instant::now().checked_add(ADMISSION_TIMEOUT).unwrap()
}
async fn run(body: impl AsyncFnOnce(&RootGate, &tokio::task::JoinHandle<()>)) {
    let stop = CancellationToken::new();
    let (gate, owner) = RootGate::start(stop.clone());
    let outcome = AssertUnwindSafe(vessel_testkit::within(
        ROOT_TEST_TIMEOUT,
        body(&gate, &owner),
    ))
    .catch_unwind()
    .await;
    stop.cancel();
    drop(gate);
    let (joined, on_time) = settle("root owner drain", owner).await;
    match outcome {
        Ok(result) => result.unwrap(),
        Err(panic) => std::panic::resume_unwind(panic),
    }
    joined.unwrap();
    assert!(on_time, "root owner drain exceeded its phase deadline");
}
#[tokio::test]
async fn checkpoint_root_admission_aliases_share_one_slot_and_stop_retains_its_owner() {
    let temp = tempfile::tempdir().unwrap();
    let one = scope(&temp.path().join("one"));
    let two = scope(&temp.path().join("two"));
    run(async |gate, owner| {
        let lease = phase("first admission", gate.claim(Arc::clone(&one), deadline()))
            .await
            .unwrap();
        assert!(matches!(
            phase("alias refusal", gate.claim(Arc::clone(&one), deadline())).await,
            Err(RootRefusal::Busy)
        ));
        let other = phase("sibling admission", gate.claim(two, deadline()))
            .await
            .unwrap();
        drop(lease);
        let next = phase(
            "released admission",
            gate.claim(Arc::clone(&one), deadline()),
        )
        .await
        .unwrap();
        gate.stop.cancel();
        assert!(matches!(
            phase("stopped admission", gate.claim(one, deadline())).await,
            Err(RootRefusal::Closed)
        ));
        phase("owner closes admission", gate.requests.closed()).await;
        assert!(!owner.is_finished(), "stop cannot release owned leases");
        drop(other);
        drop(next);
    })
    .await;
}
#[tokio::test(start_paused = true)]
async fn checkpoint_root_admission_refuses_an_expired_or_full_queue_without_extra_waiters() {
    let temp = tempfile::tempdir().unwrap();
    let root = scope(temp.path());
    run(async |gate, _owner| {
        assert!(matches!(
            gate.claim(Arc::clone(&root), Instant::now()).await,
            Err(RootRefusal::TimedOut)
        ));
        let mut replies = Vec::new();
        for _slot in 0..ADMISSION_QUEUE {
            let (reply, received) = oneshot::channel();
            gate.requests
                .try_send(Request {
                    ancestry: Arc::clone(&root),
                    deadline: deadline(),
                    reply,
                })
                .unwrap();
            replies.push(received);
        }
        assert!(matches!(
            gate.claim(root, deadline()).await,
            Err(RootRefusal::Busy)
        ));
        tokio::time::advance(ADMISSION_TIMEOUT).await;
        for reply in replies {
            assert!(matches!(
                phase("expired reply drain", reply).await.unwrap(),
                Err(RootRefusal::TimedOut)
            ));
        }
    })
    .await;
}
#[tokio::test]
async fn checkpoint_nested_scopes_conflict_in_both_orders_without_locking_siblings() {
    let temp = tempfile::tempdir().unwrap();
    let parent = scope(&temp.path().join("parent"));
    let child = scope(&temp.path().join("parent/child"));
    let sibling = scope(&temp.path().join("sibling"));
    run(async |gate, _owner| {
        let nested = phase(
            "nested admission",
            gate.claim(Arc::clone(&child), deadline()),
        )
        .await
        .unwrap();
        assert!(matches!(
            phase(
                "parent refusal",
                gate.claim(Arc::clone(&parent), deadline())
            )
            .await,
            Err(RootRefusal::Busy)
        ));
        let separate = phase("unrelated admission", gate.claim(sibling, deadline()))
            .await
            .unwrap();
        drop(nested);
        let outer = phase("parent admission", gate.claim(parent, deadline()))
            .await
            .unwrap();
        assert!(matches!(
            phase("child refusal", gate.claim(child, deadline())).await,
            Err(RootRefusal::Busy)
        ));
        drop(outer);
        drop(separate);
    })
    .await;
}
