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
        move |_key, _cancel| {
            entered.lock().unwrap().take().unwrap().send(()).unwrap();
            held.lock()
                .unwrap()
                .take()
                .unwrap()
                .recv()
                .map_err(|_closed| CheckpointError::StaleCapture)?;
            FileImage::new(Vec::new())
        },
        move |_key, _image| {
            let _previous = writes.fetch_add(1, Ordering::SeqCst);
            async { Ok(()) }
        },
        move |_key, disposition| {
            assert_eq!(disposition, Disposition::Discard);
            cleaned.lock().unwrap().take().unwrap().send(()).unwrap();
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
    assert_eq!(
        client
            .capture(capture_key, ReservedBytes::new(1).unwrap())
            .await,
        Err(CheckpointError::Busy)
    );
    release.send(()).unwrap();
    finished.await.unwrap();
    assert_eq!(published.load(Ordering::SeqCst), 0);
    drop(client);
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
        |_key, _cancel| FileImage::new(Vec::new()),
        move |_key, _image| {
            let release = release.lock().unwrap().take().unwrap();
            entered.lock().unwrap().take().unwrap().send(()).unwrap();
            async move {
                release.await.unwrap();
                Ok(())
            }
        },
        move |_key, disposition| {
            assert_eq!(disposition, Disposition::Discard);
            cleaned.lock().unwrap().take().unwrap().send(()).unwrap();
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
        |_key, _cancel| FileImage::new(Vec::new()),
        move |_key, phase| {
            let held = match phase {
                Publication::Candidate(image) => {
                    assert!(image.entries().is_empty());
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
        |_key, _cancel| FileImage::new(Vec::new()),
        |_key, phase| async move {
            match phase {
                Publication::Candidate(image) => {
                    assert!(image.entries().is_empty());
                    Ok(())
                }
                Publication::Activate => Err(CheckpointError::FeatureUnavailable),
            }
        },
        move |_key, disposition| {
            assert_eq!(disposition, Disposition::Recovery);
            cleaned.lock().unwrap().take().unwrap().send(()).unwrap();
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
