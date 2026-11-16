//! Pure admission barriers never claim terminal or filesystem cleanup proof.
use super::*;
const ROOT_TEST_TIMEOUT: Duration = Duration::from_secs(30);
use vessel_proto::BlobHash;
fn root(byte: u8) -> PhysicalRoot {
    PhysicalRoot::new(BlobHash::from_bytes([byte; 32]))
}
fn deadline() -> Instant {
    Instant::now().checked_add(ADMISSION_TIMEOUT).unwrap()
}
#[tokio::test]
async fn checkpoint_root_admission_aliases_share_one_slot_and_stop_retains_its_owner() {
    vessel_testkit::within(ROOT_TEST_TIMEOUT, async {
        let stop = CancellationToken::new();
        let (gate, owner) = RootGate::start(stop.clone());
        let lease = gate.claim(root(1), deadline()).await.unwrap();
        assert!(matches!(
            gate.claim(root(1), deadline()).await,
            Err(RootRefusal::Busy)
        ));
        let other = gate.claim(root(2), deadline()).await.unwrap();
        drop(lease);
        let next = gate.claim(root(1), deadline()).await.unwrap();
        stop.cancel();
        assert!(matches!(
            gate.claim(root(3), deadline()).await,
            Err(RootRefusal::Closed)
        ));
        gate.requests.closed().await;
        assert!(!owner.is_finished(), "stop cannot release owned leases");
        drop(other);
        drop(next);
        drop(gate);
        owner.await.unwrap();
    })
    .await
    .unwrap();
}
#[tokio::test(start_paused = true)]
async fn checkpoint_root_admission_refuses_an_expired_or_full_queue_without_extra_waiters() {
    vessel_testkit::within(ROOT_TEST_TIMEOUT, async {
        let (gate, owner) = RootGate::start(CancellationToken::new());
        assert!(matches!(
            gate.claim(root(1), Instant::now()).await,
            Err(RootRefusal::TimedOut)
        ));
        let mut replies = Vec::new();
        for _slot in 0..ADMISSION_QUEUE {
            let (reply, received) = oneshot::channel();
            gate.requests
                .try_send(Request {
                    root: root(1),
                    deadline: deadline(),
                    reply,
                })
                .unwrap();
            replies.push(received);
        }
        assert!(matches!(
            gate.claim(root(2), deadline()).await,
            Err(RootRefusal::Busy)
        ));
        tokio::time::advance(ADMISSION_TIMEOUT).await;
        for reply in replies {
            assert!(matches!(reply.await.unwrap(), Err(RootRefusal::TimedOut)));
        }
        drop(gate);
        owner.await.unwrap();
    })
    .await
    .unwrap();
}
