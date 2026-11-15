use super::{capture::Captures, quota::ReservedBytes};
use tokio::{sync::oneshot, time::Instant};
use vessel_proto::{
    EventSeq, SessionId, WorktreeId,
    checkpoint::{
        Boundary, CHECKPOINT_CAPTURE_DEADLINE, CHECKPOINT_NODE_BYTES, CHECKPOINT_WORKERS,
        CaptureGeneration, CaptureId, CaptureKey, CheckpointError,
    },
};

fn key(worktree: WorktreeId) -> CaptureKey {
    CaptureKey {
        capture: CaptureId::random(),
        session: SessionId::random(),
        worktree,
        boundary: Boundary::BeforeTurn {
            prompt: EventSeq::new(1),
        },
        generation: CaptureGeneration::FIRST,
    }
}
#[tokio::test]
async fn deadline_releases_actor_but_keeps_worker_root_and_reservation() {
    let mut captures = Captures::default();
    let key = key(WorktreeId::random());
    let now = Instant::now();
    let deadline = now + CHECKPOINT_CAPTURE_DEADLINE;
    let (reply, received) = oneshot::channel();
    captures
        .admit(
            key,
            deadline,
            ReservedBytes::new(CHECKPOINT_NODE_BYTES).unwrap(),
            reply,
        )
        .unwrap();
    captures.expire(deadline);
    assert_eq!(
        received.await.unwrap(),
        Err(CheckpointError::CaptureTimedOut)
    );
    assert!(captures.occupied(key.worktree));
    assert_eq!(captures.reserved(), CHECKPOINT_NODE_BYTES);
    assert_eq!(
        captures.published(key, deadline),
        Err(CheckpointError::StaleCapture)
    );
    captures.cleaned(key).unwrap();
    assert!(!captures.occupied(key.worktree));
    assert_eq!(captures.reserved(), 0);
}
#[tokio::test]
async fn publication_and_cleanup_never_reuse_a_different_generation() {
    let mut captures = Captures::default();
    let key = key(WorktreeId::random());
    let now = Instant::now();
    let (reply, received) = oneshot::channel();
    captures
        .admit(
            key,
            now + CHECKPOINT_CAPTURE_DEADLINE,
            ReservedBytes::new(1).unwrap(),
            reply,
        )
        .unwrap();
    let mut wrong = key;
    wrong.generation = wrong.generation.next().unwrap();
    assert_eq!(
        captures.published(wrong, now),
        Err(CheckpointError::StaleCapture)
    );
    assert_eq!(captures.cleaned(wrong), Err(CheckpointError::StaleCapture));
    captures.published(key, now).unwrap();
    assert_eq!(received.await.unwrap(), Ok(key));
    captures.cleaned(key).unwrap();
}
#[tokio::test]
async fn stuck_workers_cannot_spawn_replacements_or_prematurely_return_quota() {
    let mut captures = Captures::default();
    let now = Instant::now();
    let mut keys = Vec::new();
    for _ in 0..CHECKPOINT_WORKERS {
        let key = key(WorktreeId::random());
        let (reply, _received) = oneshot::channel();
        captures
            .admit(key, now, ReservedBytes::new(1).unwrap(), reply)
            .unwrap();
        keys.push(key);
    }
    captures.expire(now);
    let (reply, _received) = oneshot::channel();
    assert_eq!(
        captures.admit(
            key(WorktreeId::random()),
            now,
            ReservedBytes::new(1).unwrap(),
            reply
        ),
        Err(CheckpointError::Busy)
    );
    assert_eq!(
        captures.reserved(),
        u64::try_from(CHECKPOINT_WORKERS).unwrap()
    );
    for key in keys {
        captures.cleaned(key).unwrap();
    }
    assert_eq!(captures.reserved(), 0);
}
#[test]
#[ignore = "skeleton: requires the asynchronous worker and publication seam"]
fn held_filesystem_io_does_not_delay_prompt_after_capture_timeout() {
    // Proof obligation: implement held-I/O barrier fixture.
}

#[tokio::test]
async fn an_abandoned_actor_cannot_authorize_later_ready_activation() {
    let mut captures = Captures::default();
    let key = key(WorktreeId::random());
    let now = Instant::now();
    let (reply, received) = oneshot::channel();
    captures
        .admit(
            key,
            now + CHECKPOINT_CAPTURE_DEADLINE,
            ReservedBytes::new(1).unwrap(),
            reply,
        )
        .unwrap();
    drop(received);
    assert_eq!(
        captures.published(key, now),
        Err(CheckpointError::StaleCapture)
    );
    assert!(captures.occupied(key.worktree));
    assert_eq!(captures.reserved(), 1);
    captures.cleaned(key).unwrap();
    assert_eq!(captures.reserved(), 0);
}
