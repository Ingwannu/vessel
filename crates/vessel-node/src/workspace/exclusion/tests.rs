//! No sleeps: leases and replies form the synchronization barriers.
use super::*;
use vessel_proto::{BlobHash, checkpoint::CHECKPOINT_CAPTURE_DEADLINE};
fn root(byte: u8) -> PhysicalRoot {
    PhysicalRoot::new(BlobHash::from_bytes([byte; 32]))
}
fn deadline() -> Instant {
    Instant::now()
        .checked_add(CHECKPOINT_CAPTURE_DEADLINE)
        .unwrap()
}
#[tokio::test]
async fn checkpoint_root_lease_excludes_aliases_and_drop_allows_the_next_writer() {
    let (gate, owner) = RootGate::start(BTreeSet::new()).unwrap();
    let lease = gate.claim(root(1), deadline()).await.unwrap();
    assert_eq!(lease.root(), root(1));
    assert!(matches!(
        gate.claim(root(1), deadline()).await,
        Err(RootRefusal::Busy)
    ));
    let independent = gate.claim(root(2), deadline()).await.unwrap();
    drop(lease);
    let next = gate.claim(root(1), deadline()).await.unwrap();
    drop(independent);
    drop(next);
    drop(gate);
    owner.await.unwrap();
}
#[tokio::test]
async fn checkpoint_recovery_is_pinned_until_explicit_verified_resolution() {
    let (gate, owner) = RootGate::start(BTreeSet::from([root(1)])).unwrap();
    assert!(matches!(
        gate.claim(root(1), deadline()).await,
        Err(RootRefusal::RecoveryPending)
    ));
    assert_eq!(
        gate.recovered(root(2), deadline()).await,
        Err(RootRefusal::NotRecovering)
    );
    gate.recovered(root(1), deadline()).await.unwrap();
    let lease = gate.claim(root(1), deadline()).await.unwrap();
    drop(lease);
    drop(gate);
    owner.await.unwrap();
}
#[tokio::test(start_paused = true)]
async fn checkpoint_expired_admission_does_not_create_a_lease() {
    let (gate, owner) = RootGate::start(BTreeSet::new()).unwrap();
    assert!(matches!(
        gate.claim(root(1), Instant::now()).await,
        Err(RootRefusal::TimedOut)
    ));
    let lease = gate.claim(root(1), deadline()).await.unwrap();
    drop(lease);
    drop(gate);
    owner.await.unwrap();
}
#[tokio::test]
async fn checkpoint_dropped_reply_releases_its_undelivered_lease() {
    let mut active = BTreeMap::new();
    let mut recovering = BTreeSet::new();
    let released = Arc::new(Notify::new());
    let (reply, received) = oneshot::channel();
    drop(received);
    admit(
        Request::Claim {
            root: root(1),
            deadline: deadline(),
            reply,
        },
        &mut active,
        &mut recovering,
        &released,
    );
    released.notified().await;
    prune(&mut active);
    assert!(active.is_empty());
}
#[tokio::test]
async fn checkpoint_owner_drains_an_admitted_lease_after_the_last_client_drops() {
    let (gate, mut owner) = RootGate::start(BTreeSet::new()).unwrap();
    let lease = gate.claim(root(1), deadline()).await.unwrap();
    drop(gate);
    assert!(timeout_at(Instant::now(), &mut owner).await.is_err());
    drop(lease);
    owner.await.unwrap();
}

#[tokio::test]
async fn checkpoint_inflight_writer_retains_root_even_if_its_blocking_handle_is_aborted() {
    let (gate, owner) = RootGate::start(BTreeSet::new()).unwrap();
    let lease = gate.claim(root(1), deadline()).await.unwrap();
    let (entered, ready) = oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    let worker = tokio::task::spawn_blocking(move || {
        let _lease = lease;
        entered.send(()).unwrap();
        held.recv().unwrap();
    });
    ready.await.unwrap();
    worker.abort();
    assert!(matches!(
        gate.claim(root(1), deadline()).await,
        Err(RootRefusal::Busy)
    ));
    release.send(()).unwrap();
    worker.await.unwrap();
    let next = gate.claim(root(1), deadline()).await.unwrap();
    drop(next);
    drop(gate);
    owner.await.unwrap();
}
#[tokio::test]
async fn checkpoint_full_admission_queue_refuses_without_spawning_a_waiter() {
    let (requests, _held) = mpsc::channel(CHECKPOINT_QUEUE);
    let gate = RootGate { requests };
    for _ in 0..CHECKPOINT_QUEUE {
        let (reply, _received) = oneshot::channel();
        gate.requests
            .try_send(Request::Claim {
                root: root(1),
                deadline: deadline(),
                reply,
            })
            .unwrap();
    }
    assert!(matches!(
        gate.claim(root(1), deadline()).await,
        Err(RootRefusal::Busy)
    ));
}
