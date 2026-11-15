//! The intake decides driver exclusion; recovery/authority remain owner checks.
use super::*;

#[tokio::test]
async fn cancelling_pre_effect_admission_rolls_back_the_acquired_lease_and_driver_slot() {
    let (_temp, _dir, store, intake) = fixture().await;
    let intake = Arc::new(intake);
    let count = Arc::new(AtomicUsize::new(0));
    let request = candidate("cancel-before-admit", 7);
    let identity = request.identity.clone();
    let (prepared, receive) = tokio::sync::oneshot::channel();
    let task = tokio::spawn({
        let intake = intake.clone();
        let store = store.clone();
        let count = count.clone();
        async move {
            intake
                .admit(&store, request, |_receipt| async move {
                    let lease = reserve(&count)?;
                    prepared.send(()).unwrap();
                    std::future::pending::<()>().await;
                    Ok(lease)
                })
                .await
        }
    });
    receive.await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    task.abort();
    assert!(matches!(task.await, Err(error) if error.is_cancelled()));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(intake.slots.available_permits(), MAX_ACTIVE_CREATIONS);
    assert!(intake.active.lock().is_empty());
    assert!(store.creation(&identity).await.unwrap().is_none());
    store.close().await;
}

#[tokio::test]
async fn refused_authority_and_failed_durable_admission_release_only_their_own_lease() {
    let (_temp, _dir, store, intake) = fixture().await;
    let request = candidate("revoked", 7);
    let identity = request.identity.clone();
    assert!(matches!(
        intake
            .admit(&store, request, |_receipt| std::future::ready(
                Err::<(), _>(Refused)
            ))
            .await,
        Err(IntakeError::Prepare(Refused))
    ));
    assert!(store.creation(&identity).await.unwrap().is_none());
    let original = candidate("claimed", 7);
    let mut collision = candidate("collision", 7);
    collision.receipt = Receipt::admitted(
        Kind::Agent,
        original.receipt.snapshot().reserved.clone(),
        original.receipt.fingerprint(),
    )
    .unwrap();
    let _original = store.admit_creation(original).await.unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    assert!(matches!(
        intake
            .admit(&store, collision, |_receipt| std::future::ready(reserve(
                &count
            )))
            .await,
        Err(IntakeError::Store(StoreError::Creation(
            CreationError::ResourceClaimed
        )))
    ));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(intake.slots.available_permits(), MAX_ACTIVE_CREATIONS);
    assert!(intake.active.lock().is_empty());
    store.close().await;
}

#[tokio::test]
async fn idle_accepted_reuses_its_ids_but_committed_begin_and_unknown_never_reacquire() {
    let (_temp, dir, store, intake) = fixture().await;
    let accepted = store.admit_creation(candidate("restart", 7)).await.unwrap();
    store.close().await;
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let count = Arc::new(AtomicUsize::new(0));
    let resumed = intake
        .admit(&store, candidate("restart", 7), |receipt| {
            assert_eq!(receipt, accepted);
            std::future::ready(reserve(&count))
        })
        .await
        .unwrap();
    assert!(matches!(&resumed, Admitted::Start(_)));
    let marker = store
        .advance_creation(
            accepted.snapshot().reserved.operation,
            accepted.snapshot().revision,
            Advance::Begin(Stage::Resource),
        )
        .await
        .unwrap();
    drop(resumed);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    let calls = AtomicUsize::new(0);
    let prepare = |_receipt| {
        calls.fetch_add(1, Ordering::SeqCst);
        std::future::ready(reserve(&count))
    };
    let pending = intake
        .admit(&store, candidate("restart", 7), prepare)
        .await
        .unwrap();
    assert!(matches!(pending, Admitted::Observe(_)));
    assert_eq!(snapshot(&pending), &marker);
    store.close().await;
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let unknown = intake
        .admit(&store, candidate("restart", 7), prepare)
        .await
        .unwrap();
    assert!(matches!(&unknown, Admitted::Observe(_)));
    assert!(matches!(
        snapshot(&unknown).snapshot().phase,
        Phase::Failed {
            certainty: Certainty::Unknown,
            ..
        }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    store.close().await;
}

#[tokio::test]
async fn known_prompt_retry_continues_registered_identity_without_new_child_count() {
    let (_temp, _dir, store, intake) = fixture().await;
    let mut request = candidate("prompt", 7);
    let mut reservation = request.receipt.snapshot().reserved.clone();
    reservation.first_input = FirstInput::Identified {
        message: vessel_proto::timeline::ClientMessageId::random(),
    };
    request.receipt = Receipt::admitted(
        Kind::Agent,
        reservation.clone(),
        request.receipt.fingerprint(),
    )
    .unwrap();
    let mut receipt = store.admit_creation(request).await.unwrap();
    for update in [
        Advance::Begin(Stage::Resource),
        Advance::Finished(Stage::Resource),
        Advance::Begin(Stage::Prompt),
        Advance::Fail {
            code: ErrorCode::Conflict,
            certainty: Certainty::Known,
        },
    ] {
        receipt = store
            .advance_creation(reservation.operation, receipt.snapshot().revision, update)
            .await
            .unwrap();
    }
    let retry = intake
        .admit(&store, candidate("prompt", 7), |stored| {
            assert_eq!(stored.snapshot().reserved, reservation);
            assert!(matches!(
                stored.snapshot().phase,
                Phase::Failed {
                    stage: Stage::Prompt,
                    certainty: Certainty::Known,
                    ..
                }
            ));
            // The real adapter must recognize the registered child here. No count
            // acquisition is needed to retry its canonical first-message identity.
            std::future::ready(Ok::<(), Refused>(()))
        })
        .await
        .unwrap();
    assert!(matches!(&retry, Admitted::Start(_)));
    drop(retry);
    store.close().await;
}
