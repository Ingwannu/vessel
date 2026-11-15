//! No sleeps: leases and replies form the synchronization barriers.
use super::*;
use vessel_proto::{BlobHash, checkpoint::CHECKPOINT_CAPTURE_DEADLINE};
fn root(byte: u8) -> PhysicalRoot {
    PhysicalRoot::new(BlobHash::from_bytes([byte; 32]))
}
fn recovering(root: PhysicalRoot) -> (RecoveryRoots, vessel_proto::checkpoint::RestoreId) {
    let mut roots = RecoveryRoots::default();
    let restore = vessel_proto::checkpoint::RestoreId::random();
    roots.add(root, WorktreeId::random(), restore).unwrap();
    (roots, restore)
}
fn deadline() -> Instant {
    Instant::now()
        .checked_add(CHECKPOINT_CAPTURE_DEADLINE)
        .unwrap()
}
#[tokio::test]
async fn checkpoint_root_lease_excludes_aliases_and_drop_allows_the_next_writer() {
    let (gate, owner) = RootGate::start(RecoveryRoots::default(), CancellationToken::new());
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
    let (recovering, restore) = recovering(root(1));
    let (gate, owner) = RootGate::start(recovering, CancellationToken::new());
    assert!(matches!(
        gate.claim(root(1), deadline()).await,
        Err(RootRefusal::RecoveryPending)
    ));
    assert!(matches!(
        gate.claim_recovery(root(2), restore, deadline()).await,
        Err(RootRefusal::NotRecovering)
    ));
    let recovery = gate
        .claim_recovery(root(1), restore, deadline())
        .await
        .unwrap();
    for _ in 0..2 {
        gate.resolve_recovery(&recovery, restore, deadline())
            .await
            .unwrap();
    }
    assert!(
        matches!(
            gate.claim(root(1), deadline()).await,
            Err(RootRefusal::Busy)
        ),
        "settlement does not release an owned physical guard"
    );
    drop(recovery);
    let lease = gate.claim(root(1), deadline()).await.unwrap();
    drop(lease);
    drop(gate);
    owner.await.unwrap();
}

#[tokio::test]
async fn checkpoint_replaced_folder_cannot_bypass_its_managed_recovery_namespace() {
    let worktree = WorktreeId::random();
    let independent = WorktreeId::random();
    let mut recovering = RecoveryRoots::default();
    recovering
        .add(
            root(1),
            worktree,
            vessel_proto::checkpoint::RestoreId::random(),
        )
        .unwrap();
    let (gate, owner) = RootGate::start(recovering, CancellationToken::new());
    assert!(matches!(
        gate.claim_registered(root(2), BTreeSet::from([worktree]), deadline())
            .await,
        Err(RootRefusal::RecoveryPending)
    ));
    assert!(matches!(
        gate.claim_registered(root(1), BTreeSet::from([independent]), deadline())
            .await,
        Err(RootRefusal::RecoveryPending)
    ));
    let other = gate
        .claim_registered(root(3), BTreeSet::from([independent]), deadline())
        .await
        .unwrap();
    assert_eq!(
        other.root(),
        root(3),
        "the actual physical key remains the lease authority"
    );
    drop(other);
    drop(gate);
    owner.await.unwrap();
}

#[tokio::test]
async fn checkpoint_operation_resolution_keeps_other_alias_pins_and_rejects_a_stale_lease() {
    use vessel_proto::checkpoint::RestoreId;
    let one = RestoreId::random();
    let two = RestoreId::random();
    let first = WorktreeId::random();
    let second = WorktreeId::random();
    let (gate, owner) = RootGate::start(RecoveryRoots::default(), CancellationToken::new());
    let writer = gate.claim(root(1), deadline()).await.unwrap();
    gate.pin_recovery(&writer, first, one, deadline())
        .await
        .unwrap();
    gate.pin_recovery(&writer, second, two, deadline())
        .await
        .unwrap();
    drop(writer);
    let recovering = gate.claim_recovery(root(1), one, deadline()).await.unwrap();
    assert_eq!(
        gate.resolve_recovery(&recovering, two, deadline()).await,
        Err(RootRefusal::StaleLease)
    );
    gate.resolve_recovery(&recovering, one, deadline())
        .await
        .unwrap();
    let stale = Arc::clone(&recovering.release);
    drop(recovering);
    assert!(matches!(
        gate.claim(root(1), deadline()).await,
        Err(RootRefusal::RecoveryPending)
    ));
    assert!(matches!(
        gate.claim_registered(root(2), BTreeSet::from([second]), deadline())
            .await,
        Err(RootRefusal::RecoveryPending)
    ));
    let recovering = gate.claim_recovery(root(1), two, deadline()).await.unwrap();
    let at = deadline();
    assert_eq!(
        gate.request(at, |reply| Request::ResolveRecovery {
            root: root(1),
            restore: two,
            held: stale,
            deadline: at,
            reply
        })
        .await,
        Err(RootRefusal::StaleLease)
    );
    gate.resolve_recovery(&recovering, two, deadline())
        .await
        .unwrap();
    drop(recovering);
    let normal = gate.claim(root(1), deadline()).await.unwrap();
    drop(normal);
    drop(gate);
    owner.await.unwrap();
}
#[tokio::test(start_paused = true)]
async fn checkpoint_expired_admission_does_not_create_a_lease() {
    let (gate, owner) = RootGate::start(RecoveryRoots::default(), CancellationToken::new());
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
    let mut state = State {
        active: BTreeMap::new(),
        recovering: RecoveryRoots::default(),
    };
    let released = Arc::new(Notify::new());
    let (reply, received) = oneshot::channel();
    drop(received);
    admit(
        Request::Claim {
            worktrees: BTreeSet::new(),
            root: root(1),
            deadline: deadline(),
            reply,
        },
        &mut state,
        &released,
    );
    released.notified().await;
    prune(&mut state.active);
    assert!(state.active.is_empty());
}
#[tokio::test]
async fn checkpoint_owner_drains_an_admitted_lease_after_the_last_client_drops() {
    let (gate, mut owner) = RootGate::start(RecoveryRoots::default(), CancellationToken::new());
    let lease = gate.claim(root(1), deadline()).await.unwrap();
    drop(gate);
    assert!(timeout_at(Instant::now(), &mut owner).await.is_err());
    drop(lease);
    owner.await.unwrap();
}

#[tokio::test]
async fn checkpoint_inflight_writer_retains_root_even_if_its_blocking_handle_is_aborted() {
    let (gate, owner) = RootGate::start(RecoveryRoots::default(), CancellationToken::new());
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
    let gate = RootGate {
        requests,
        stop: CancellationToken::new(),
    };
    for _ in 0..CHECKPOINT_QUEUE {
        let (reply, _received) = oneshot::channel();
        gate.requests
            .try_send(Request::Claim {
                worktrees: BTreeSet::new(),
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

#[tokio::test]
async fn checkpoint_shutdown_refuses_new_writers_but_drains_the_admitted_root() {
    let stop = CancellationToken::new();
    let (gate, mut owner) = RootGate::start(RecoveryRoots::default(), stop.clone());
    let lease = gate.claim(root(1), deadline()).await.unwrap();
    stop.cancel();
    assert!(matches!(
        gate.claim(root(2), deadline()).await,
        Err(RootRefusal::Closed)
    ));
    assert!(timeout_at(Instant::now(), &mut owner).await.is_err());
    drop(lease);
    owner.await.unwrap();
}
