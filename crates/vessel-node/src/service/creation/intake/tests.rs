//! Real receipt writer, bounded local driver book, and an opaque count-owner fake.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use vessel_proto::creation::{
    Advance, FirstInput, Key, Kind, Reservation, Resource, Stage, WorkspaceClaim,
};
use vessel_proto::{BlobHash, ErrorCode, SessionId, WorkspaceId};
use vessel_store::{DataDir, StoreConfig};

mod count;
mod recovery;

async fn fixture() -> (tempfile::TempDir, DataDir, Store, Intake) {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    (temp, dir, store, Intake::default())
}

fn candidate(key: &str, fingerprint: u8) -> CreationAdmission {
    CreationAdmission {
        identity: CreationIdentity::new(Kind::Agent, Key::parse(key).unwrap()),
        receipt: Receipt::admitted(
            Kind::Agent,
            Reservation {
                operation: WorkspaceOperationId::random(),
                workspace: WorkspaceId::random(),
                workspace_claim: WorkspaceClaim::Existing,
                resource: Some(Resource::Agent {
                    session: SessionId::random(),
                }),
                first_input: FirstInput::Absent,
            },
            Fingerprint::new(BlobHash::from_bytes([fingerprint; 32])),
        )
        .unwrap(),
    }
}

fn snapshot<L>(admitted: &Admitted<L>) -> &Receipt {
    match admitted {
        Admitted::Observe(receipt) => receipt,
        Admitted::Start(driver) => &driver.receipt,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Refused;

struct CountLease(Arc<AtomicUsize>);
impl Drop for CountLease {
    fn drop(&mut self) {
        assert_eq!(self.0.fetch_sub(1, Ordering::SeqCst), 1);
    }
}
fn reserve(count: &Arc<AtomicUsize>) -> Result<CountLease, Refused> {
    let _previous = count
        .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_full| Refused)?;
    Ok(CountLease(Arc::clone(count)))
}

#[tokio::test]
async fn concurrent_identical_last_count_slot_attaches_without_a_second_reservation() {
    let (_temp, _dir, store, intake) = fixture().await;
    let count = Arc::new(AtomicUsize::new(0));
    let calls = AtomicUsize::new(0);
    let first = candidate("same", 7);
    let second = candidate("same", 7);
    let candidates = [
        first.receipt.snapshot().reserved.clone(),
        second.receipt.snapshot().reserved.clone(),
    ];
    let prepare = |_receipt: Receipt| {
        let _previous = calls.fetch_add(1, Ordering::SeqCst);
        std::future::ready(reserve(&count))
    };
    let (one, two) = tokio::join!(
        intake.admit(&store, first, prepare),
        intake.admit(&store, second, prepare),
    );
    let one = one.unwrap();
    let two = two.unwrap();
    assert!(matches!(
        (&one, &two),
        (Admitted::Start(_), Admitted::Observe(_)) | (Admitted::Observe(_), Admitted::Start(_))
    ));
    assert_eq!(snapshot(&one), snapshot(&two));
    assert!(candidates.contains(&snapshot(&one).snapshot().reserved));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    drop(one);
    drop(two);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert!(intake.active.lock().is_empty());
    assert_eq!(intake.slots.available_permits(), MAX_ACTIVE_CREATIONS);
    store.close().await;
}

#[tokio::test]
async fn active_capacity_keeps_existing_and_conflicting_keys_queryable_without_prepare() {
    let (_temp, _dir, store, intake) = fixture().await;
    let mut drivers = Vec::new();
    for index in 0..MAX_ACTIVE_CREATIONS {
        let admitted = intake
            .admit(&store, candidate(&format!("held-{index}"), 7), |_receipt| {
                std::future::ready(Ok::<(), Refused>(()))
            })
            .await
            .unwrap();
        drivers.push(admitted);
    }
    assert_eq!(intake.active.lock().len(), MAX_ACTIVE_CREATIONS);
    let calls = AtomicUsize::new(0);
    let prepare = |_receipt| {
        let _previous = calls.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok::<(), Refused>(()))
    };
    let overflow = candidate("overflow", 7);
    let identity = overflow.identity.clone();
    assert!(matches!(
        intake.admit(&store, overflow, prepare).await,
        Err(IntakeError::Capacity)
    ));
    assert!(store.creation(&identity).await.unwrap().is_none());
    let duplicate = intake
        .admit(&store, candidate("held-0", 7), prepare)
        .await
        .unwrap();
    assert!(matches!(duplicate, Admitted::Observe(_)));
    assert!(matches!(
        intake.admit(&store, candidate("held-0", 8), prepare).await,
        Err(IntakeError::Store(StoreError::Creation(
            CreationError::IntentConflict
        )))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    drop(drivers);
    assert!(intake.active.lock().is_empty());
    store.close().await;
}

#[tokio::test(start_paused = true)]
async fn admission_wait_is_finite_and_never_acquires_a_resource_while_waiting() {
    let (_temp, _dir, store, intake) = fixture().await;
    let _held = intake.admission.lock().await;
    let calls = AtomicUsize::new(0);
    let request = candidate("waiter", 7);
    let identity = request.identity.clone();
    let result = intake
        .admit(&store, request, |_receipt| {
            let _previous = calls.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Ok::<(), Refused>(()))
        })
        .await;
    assert!(matches!(result, Err(IntakeError::Busy)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(intake.slots.available_permits(), MAX_ACTIVE_CREATIONS);
    assert!(store.creation(&identity).await.unwrap().is_none());
    store.close().await;
}
