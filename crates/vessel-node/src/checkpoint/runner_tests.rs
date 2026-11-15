//! Barriers retain real blocking work while the coordinator acknowledges expiry.
use super::{
    quota::ReservedBytes,
    runner::{self, Disposition, Publication},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::oneshot;
use vessel_proto::{
    EventSeq, SessionId, WorktreeId,
    checkpoint::{
        Boundary, CHECKPOINT_CAPTURE_DEADLINE, CaptureGeneration, CaptureId, CaptureKey,
        CheckpointError, FileImage,
    },
};

fn key() -> CaptureKey {
    CaptureKey {
        capture: CaptureId::random(),
        session: SessionId::random(),
        worktree: WorktreeId::random(),
        boundary: Boundary::BeforeTurn {
            prompt: EventSeq::new(1),
        },
        generation: CaptureGeneration::FIRST,
    }
}
#[tokio::test(start_paused = true)]
async fn held_read_io_releases_prompt_and_late_candidate_never_publishes() {
    let (release, held) = std::sync::mpsc::channel();
    let held = std::sync::Mutex::new(Some(held));
    let (entered, entry) = oneshot::channel();
    let entered = std::sync::Mutex::new(Some(entered));
    let published = Arc::new(AtomicUsize::new(0));
    let writes = Arc::clone(&published);
    let (cleaned, finished) = oneshot::channel();
    let cleaned = std::sync::Mutex::new(Some(cleaned));
    let (client, owner) = runner::start(
        move |_key, _physical, _cancel| {
            entered.lock().unwrap().take().unwrap().send(()).unwrap();
            held.lock()
                .unwrap()
                .take()
                .unwrap()
                .recv()
                .map_err(|_closed| CheckpointError::StaleCapture)?;
            empty()
        },
        move |_key, phase| {
            match phase {
                Publication::Reserve(bytes) => assert_eq!(bytes.get(), 1),
                Publication::Candidate(_image) => {
                    let _previous = writes.fetch_add(1, Ordering::SeqCst);
                }
                Publication::Activate => panic!("expired read cannot activate"),
            }
            async { Ok(()) }
        },
        move |_key, disposition| {
            assert!(matches!(disposition, Disposition::Discard(_)));
            cleaned.lock().unwrap().take().unwrap().send(()).unwrap();
            Ok(())
        },
    );
    let capture_key = key();
    let physical = super::capture::model_root(capture_key.worktree);
    let call = client.clone();
    let prompt = tokio::spawn(async move {
        call.capture_at_root(
            physical,
            capture_key,
            ReservedBytes::new(1).unwrap(),
            tokio::time::Instant::now() + CHECKPOINT_CAPTURE_DEADLINE,
        )
        .await
    });
    entry.await.unwrap();
    tokio::time::advance(CHECKPOINT_CAPTURE_DEADLINE).await;
    assert_eq!(prompt.await.unwrap(), Err(CheckpointError::CaptureTimedOut));
    let alias = key();
    assert_ne!(alias.worktree, capture_key.worktree);
    assert_eq!(
        client
            .capture_at_root(
                physical,
                alias,
                ReservedBytes::new(1).unwrap(),
                tokio::time::Instant::now() + CHECKPOINT_CAPTURE_DEADLINE
            )
            .await,
        Err(CheckpointError::Busy),
        "a different logical worktree cannot bypass a held physical capture guard",
    );
    release.send(()).unwrap();
    finished.await.unwrap();
    assert_eq!(published.load(Ordering::SeqCst), 0);
    drop(client);
    owner.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn dropping_all_clients_does_not_drop_the_owner_before_held_io_is_cleaned() {
    let (release, held) = std::sync::mpsc::channel();
    let held = std::sync::Mutex::new(Some(held));
    let (entered, entry) = oneshot::channel();
    let entered = std::sync::Mutex::new(Some(entered));
    let (client, owner) = runner::start(
        move |_key, _physical, _cancel| {
            entered.lock().unwrap().take().unwrap().send(()).unwrap();
            let _closed = held.lock().unwrap().take().unwrap().recv();
            empty()
        },
        |_key, _phase| async { Ok(()) },
        |_key, disposition| {
            assert!(matches!(disposition, Disposition::Discard(_)));
            Ok(())
        },
    );
    let call = client.clone();
    let actor =
        tokio::spawn(async move { call.capture(key(), ReservedBytes::new(1).unwrap()).await });
    entry.await.unwrap();
    tokio::time::advance(CHECKPOINT_CAPTURE_DEADLINE).await;
    assert_eq!(actor.await.unwrap(), Err(CheckpointError::CaptureTimedOut));
    let mut lifecycle = client.lifecycle();
    drop(client);
    while *lifecycle.borrow_and_update() == runner::Lifecycle::Accepting {
        lifecycle.changed().await.unwrap();
    }
    assert!(
        !owner.is_finished(),
        "closing callers cannot prematurely release an admitted worker"
    );
    release.send(()).unwrap();
    owner.await.unwrap();
}
#[tokio::test(start_paused = true)]
async fn held_publication_ack_cannot_revive_the_expired_lease() {
    let (hold, release) = oneshot::channel();
    let release = std::sync::Mutex::new(Some(release));
    let (entered, entry) = oneshot::channel();
    let entered = std::sync::Mutex::new(Some(entered));
    let (cleaned, finished) = oneshot::channel();
    let cleaned = std::sync::Mutex::new(Some(cleaned));
    let (client, owner) = runner::start(
        |_key, _physical, _cancel| empty(),
        move |_key, phase| {
            let release = match phase {
                Publication::Reserve(bytes) => {
                    assert_eq!(bytes.get(), 1);
                    None
                }
                Publication::Candidate(_image) => {
                    entered.lock().unwrap().take().unwrap().send(()).unwrap();
                    Some(release.lock().unwrap().take().unwrap())
                }
                Publication::Activate => panic!("expired candidate cannot activate"),
            };
            async move {
                if let Some(release) = release {
                    release.await.unwrap();
                }
                Ok(())
            }
        },
        move |_key, disposition| {
            assert!(matches!(disposition, Disposition::Discard(_)));
            cleaned.lock().unwrap().take().unwrap().send(()).unwrap();
            Ok(())
        },
    );
    let capture_key = key();
    let call = client.clone();
    let prompt = tokio::spawn(async move {
        call.capture(capture_key, ReservedBytes::new(1).unwrap())
            .await
    });
    entry.await.unwrap();
    tokio::time::advance(CHECKPOINT_CAPTURE_DEADLINE).await;
    assert_eq!(prompt.await.unwrap(), Err(CheckpointError::CaptureTimedOut));
    hold.send(()).unwrap();
    finished.await.unwrap();
    drop(client);
    owner.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn accepted_candidate_releases_actor_but_activation_retains_worker_and_root_guard() {
    let (release, held) = oneshot::channel();
    let held = std::sync::Mutex::new(Some(held));
    let (entered, entry) = oneshot::channel();
    let entered = std::sync::Mutex::new(Some(entered));
    let ready = Arc::new(AtomicUsize::new(0));
    let ready_write = Arc::clone(&ready);
    let (cleaned, finished) = oneshot::channel();
    let cleaned = std::sync::Mutex::new(Some(cleaned));
    let (client, owner) = runner::start(
        |_key, _physical, _cancel| empty(),
        move |_key, phase| {
            let held = match phase {
                Publication::Reserve(bytes) => {
                    assert_eq!(bytes.get(), 1);
                    None
                }
                Publication::Candidate(image) => {
                    assert!(image.image().entries().is_empty());
                    None
                }
                Publication::Activate => {
                    entered.lock().unwrap().take().unwrap().send(()).unwrap();
                    Some(held.lock().unwrap().take().unwrap())
                }
            };
            let ready = Arc::clone(&ready_write);
            async move {
                if let Some(held) = held {
                    held.await.unwrap();
                    let _previous = ready.fetch_add(1, Ordering::SeqCst);
                }
                Ok(())
            }
        },
        move |_key, disposition| {
            assert_eq!(disposition, Disposition::Keep);
            cleaned.lock().unwrap().take().unwrap().send(()).unwrap();
            Ok(())
        },
    );
    let key = key();
    assert_eq!(
        client.capture(key, ReservedBytes::new(1).unwrap()).await,
        Ok(key)
    );
    entry.await.unwrap();
    tokio::time::advance(CHECKPOINT_CAPTURE_DEADLINE).await;
    assert_eq!(
        ready.load(Ordering::SeqCst),
        0,
        "a candidate acknowledgement is not user-visible Ready"
    );
    assert_eq!(
        client.capture(key, ReservedBytes::new(1).unwrap()).await,
        Err(CheckpointError::Busy)
    );
    release.send(()).unwrap();
    finished.await.unwrap();
    assert_eq!(ready.load(Ordering::SeqCst), 1);
    drop(client);
    owner.await.unwrap();
}

#[tokio::test]
async fn uncertain_ready_activation_keeps_archive_for_reconciliation() {
    let (cleaned, finished) = oneshot::channel();
    let cleaned = std::sync::Mutex::new(Some(cleaned));
    let (client, owner) = runner::start(
        |_key, _physical, _cancel| empty(),
        |_key, phase| async move {
            match phase {
                Publication::Reserve(bytes) => {
                    assert_eq!(bytes.get(), 1);
                    Ok(())
                }
                Publication::Candidate(image) => {
                    assert!(image.image().entries().is_empty());
                    Ok(())
                }
                Publication::Activate => Err(CheckpointError::FeatureUnavailable),
            }
        },
        move |_key, disposition| {
            assert_eq!(disposition, Disposition::Recovery);
            cleaned.lock().unwrap().take().unwrap().send(()).unwrap();
            Ok(())
        },
    );
    let key = key();
    assert_eq!(
        client.capture(key, ReservedBytes::new(1).unwrap()).await,
        Ok(key)
    );
    finished.await.unwrap();
    drop(client);
    owner.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn held_global_reservation_never_starts_filesystem_work_after_actor_deadline() {
    let (release, held) = oneshot::channel();
    let held = std::sync::Mutex::new(Some(held));
    let (entered, entry) = oneshot::channel();
    let entered = std::sync::Mutex::new(Some(entered));
    let reads = Arc::new(AtomicUsize::new(0));
    let read_count = Arc::clone(&reads);
    let (cleaned, finished) = oneshot::channel();
    let cleaned = std::sync::Mutex::new(Some(cleaned));
    let (client, owner) = runner::start(
        move |_key, _physical, _cancel| {
            let _previous = read_count.fetch_add(1, Ordering::SeqCst);
            empty()
        },
        move |_key, phase| {
            let held = match phase {
                Publication::Reserve(bytes) => {
                    assert_eq!(bytes.get(), 1);
                    entered.lock().unwrap().take().unwrap().send(()).unwrap();
                    held.lock().unwrap().take().unwrap()
                }
                Publication::Candidate(_) | Publication::Activate => {
                    panic!("an expired reservation cannot read or activate")
                }
            };
            async move {
                held.await.unwrap();
                Ok(())
            }
        },
        move |_key, disposition| {
            assert!(matches!(disposition, Disposition::Discard(_)));
            cleaned.lock().unwrap().take().unwrap().send(()).unwrap();
            Ok(())
        },
    );
    let key = key();
    let call = client.clone();
    let actor =
        tokio::spawn(async move { call.capture(key, ReservedBytes::new(1).unwrap()).await });
    entry.await.unwrap();
    tokio::time::advance(CHECKPOINT_CAPTURE_DEADLINE).await;
    assert_eq!(actor.await.unwrap(), Err(CheckpointError::CaptureTimedOut));
    assert_eq!(
        client.capture(key, ReservedBytes::new(1).unwrap()).await,
        Err(CheckpointError::Busy)
    );
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    release.send(()).unwrap();
    finished.await.unwrap();
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    drop(client);
    owner.await.unwrap();
}

#[tokio::test]
async fn failed_cleanup_cannot_return_worker_capacity_or_its_root_guard() {
    let (client, owner) = runner::start(
        |_key, _physical, _cancel| empty(),
        |_key, _phase| async { Ok(()) },
        |_key, _disposition| Err(CheckpointError::StorageFull),
    );
    let mut faults = client.cleanup_faults();
    let key = key();
    assert_eq!(
        client.capture(key, ReservedBytes::new(1).unwrap()).await,
        Ok(key)
    );
    while faults.borrow_and_update().is_none() {
        faults.changed().await.unwrap();
    }
    assert_eq!(
        *faults.borrow(),
        Some(runner::CleanupFault {
            key,
            reason: CheckpointError::StorageFull
        })
    );
    assert_eq!(
        client.capture(key, ReservedBytes::new(1).unwrap()).await,
        Err(CheckpointError::Busy)
    );
    // Cleanup actually returned an error: shutdown may exit with durable
    // recovery charges, while an active blocked worker still cannot drain.
    drop(client);
    owner.await.unwrap();
}

fn empty() -> Result<vessel_proto::checkpoint::ObservedSnapshot, CheckpointError> {
    use vessel_proto::{
        BlobHash,
        checkpoint::{
            IgnoreFingerprint, ObservedSnapshot, PhysicalRoot, RootFingerprint, SnapshotEvidence,
        },
    };
    let evidence = SnapshotEvidence::new(
        PhysicalRoot::new(BlobHash::from_bytes([0; 32])),
        RootFingerprint::new(BlobHash::from_bytes([1; 32])),
        IgnoreFingerprint::new(BlobHash::from_bytes([2; 32])),
        Vec::new(),
    )?;
    ObservedSnapshot::new(FileImage::new(Vec::new())?, evidence)
}

#[tokio::test]
async fn checkpoint_actor_receipt_gates_ready_and_refused_or_lost_delivery_discards() {
    use vessel_session::CheckpointAdvance;
    for outcome in [
        Some(CheckpointAdvance::PromptSent),
        Some(CheckpointAdvance::PromptRefused),
        None,
    ] {
        let activated = Arc::new(AtomicUsize::new(0));
        let writes = Arc::clone(&activated);
        let (cleaned, finished) = oneshot::channel();
        let cleaned = std::sync::Mutex::new(Some(cleaned));
        let expected = match outcome {
            Some(CheckpointAdvance::PromptSent | CheckpointAdvance::TurnComplete) => {
                Disposition::Keep
            }
            Some(CheckpointAdvance::PromptRefused) | None => {
                Disposition::Discard(CheckpointError::StaleCapture)
            }
        };
        let (client, owner) = runner::start(
            |_key, _root, _cancel| empty(),
            move |_key, publication| {
                match publication {
                    Publication::Reserve(_) | Publication::Candidate(_) => {}
                    Publication::Activate => {
                        let _previous = writes.fetch_add(1, Ordering::SeqCst);
                    }
                }
                async { Ok(()) }
            },
            move |_key, disposition| {
                assert_eq!(disposition, expected);
                cleaned.lock().unwrap().take().unwrap().send(()).unwrap();
                Ok(())
            },
        );
        let key = key();
        let accepted = client
            .capture_reserved(
                super::capture::model_root(key.worktree),
                key,
                ReservedBytes::new(1).unwrap(),
                tokio::time::Instant::now() + CHECKPOINT_CAPTURE_DEADLINE,
            )
            .await
            .unwrap();
        assert_eq!(accepted.key, key);
        assert_eq!(
            activated.load(Ordering::SeqCst),
            0,
            "candidate acknowledgement alone is not actor acceptance"
        );
        match outcome {
            Some(outcome) => accepted.complete(outcome),
            None => drop(accepted),
        }
        finished.await.unwrap();
        assert_eq!(
            activated.load(Ordering::SeqCst),
            usize::from(expected == Disposition::Keep)
        );
        drop(client);
        owner.await.unwrap();
    }
}

#[tokio::test]
async fn checkpoint_next_boundary_waits_for_accepted_publication_without_spawning_another_read() {
    use vessel_session::CheckpointAdvance;
    let first = key();
    let physical = super::capture::model_root(first.worktree);
    let reads = Arc::new(AtomicUsize::new(0));
    let reader = Arc::clone(&reads);
    let (release, held) = oneshot::channel();
    let held = std::sync::Mutex::new(Some(held));
    let (entered, entry) = oneshot::channel();
    let entered = std::sync::Mutex::new(Some(entered));
    let (client, owner) = runner::start(
        move |_key, _root, _cancel| {
            let _old = reader.fetch_add(1, Ordering::SeqCst);
            empty()
        },
        move |key, phase| {
            let held = match phase {
                Publication::Activate if key == first => {
                    entered.lock().unwrap().take().unwrap().send(()).unwrap();
                    Some(held.lock().unwrap().take().unwrap())
                }
                Publication::Reserve(_) | Publication::Candidate(_) | Publication::Activate => None,
            };
            async move {
                if let Some(held) = held {
                    held.await.unwrap();
                }
                Ok(())
            }
        },
        |_key, _disposition| Ok(()),
    );
    let accepted = client
        .capture_reserved(
            physical,
            first,
            ReservedBytes::new(1).unwrap(),
            tokio::time::Instant::now() + CHECKPOINT_CAPTURE_DEADLINE,
        )
        .await
        .unwrap();
    accepted.complete(CheckpointAdvance::TurnComplete);
    entry.await.unwrap();
    let next = CaptureKey {
        capture: CaptureId::random(),
        boundary: Boundary::BeforeTurn {
            prompt: EventSeq::new(2),
        },
        generation: first.generation.next().unwrap(),
        ..first
    };
    let accepted = {
        let next_capture = client.capture_candidate(
            physical,
            next,
            tokio::time::Instant::now() + CHECKPOINT_CAPTURE_DEADLINE,
        );
        tokio::pin!(next_capture);
        assert!(futures_util::poll!(next_capture.as_mut()).is_pending());
        assert_eq!(reads.load(Ordering::SeqCst), 1);
        assert_eq!(
            client
                .wait_for_root(physical, tokio::time::Instant::now())
                .await,
            Err(CheckpointError::CaptureTimedOut)
        );
        release.send(()).unwrap();
        next_capture.await.unwrap()
    };
    assert_eq!(accepted.key, next);
    assert_eq!(reads.load(Ordering::SeqCst), 2);
    accepted.complete(CheckpointAdvance::TurnComplete);
    drop(client);
    owner.await.unwrap();
}

#[tokio::test]
async fn checkpoint_confirmed_preinsert_reservation_refusals_do_not_permanently_exhaust_workers() {
    vessel_testkit::within(std::time::Duration::from_secs(60), async {
        for failure in [CheckpointError::StorageFull, CheckpointError::StaleCapture] {
            let (client, owner) = runner::start(
                |_key, _root, _cancel| panic!("preinsert refusal must not read files"),
                move |_key, publication| {
                    assert!(matches!(publication, Publication::Reserve(_)));
                    async move { Err(failure) }
                },
                |_key, disposition| {
                    assert_eq!(disposition, Disposition::NoReservation);
                    Ok(())
                },
            );
            let first = key();
            let physical = super::capture::model_root(first.worktree);
            for _attempt in 0..3 {
                let key = CaptureKey {
                    capture: CaptureId::random(),
                    ..first
                };
                assert_eq!(
                    client
                        .capture_at_root(
                            physical,
                            key,
                            ReservedBytes::new(1).unwrap(),
                            tokio::time::Instant::now() + CHECKPOINT_CAPTURE_DEADLINE
                        )
                        .await,
                    Err(failure)
                );
                client.wait_released(physical).await.unwrap();
            }
            drop(client);
            owner.await.unwrap();
        }
    })
    .await
    .unwrap();
}
