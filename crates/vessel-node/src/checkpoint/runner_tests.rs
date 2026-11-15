//! Barriers retain real blocking work while the coordinator acknowledges expiry.
use super::{
    quota::ReservedBytes,
    runner::{self, Disposition},
};
use std::sync::{
    Arc, Barrier,
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
    let barrier = Arc::new(Barrier::new(2));
    let worker_barrier = Arc::clone(&barrier);
    let (entered, entry) = oneshot::channel();
    let entered = std::sync::Mutex::new(Some(entered));
    let published = Arc::new(AtomicUsize::new(0));
    let writes = Arc::clone(&published);
    let (cleaned, finished) = oneshot::channel();
    let cleaned = std::sync::Mutex::new(Some(cleaned));
    let (client, owner) = runner::start(
        move |_key, _cancel| {
            entered.lock().unwrap().take().unwrap().send(()).unwrap();
            worker_barrier.wait();
            FileImage::new(Vec::new())
        },
        move |_key, _image| {
            writes.fetch_add(1, Ordering::SeqCst);
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
    barrier.wait();
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
