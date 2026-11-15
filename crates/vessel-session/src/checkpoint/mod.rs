//! ADR 0077 staged turn seam. Production wiring waits for Quality B.
//!
//! Only the node owns files/storage. A full coordinator queue acknowledges Busy
//! immediately; a dropped reply is unavailable, never a reason to hold a prompt.

use tokio::sync::{mpsc, oneshot};
use vessel_proto::checkpoint::{CaptureKey, CheckpointError};

#[derive(Debug)]
struct CaptureBoundary {
    key: CaptureKey,
    reply: oneshot::Sender<Result<CaptureKey, CheckpointError>>,
}

async fn acknowledge(
    sender: &mpsc::Sender<CaptureBoundary>,
    key: CaptureKey,
) -> Result<CaptureKey, CheckpointError> {
    let (reply, received) = oneshot::channel();
    sender
        .try_send(CaptureBoundary { key, reply })
        .map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) | mpsc::error::TrySendError::Closed(_) => {
                CheckpointError::Busy
            }
        })?;
    received
        .await
        .map_err(|_closed| CheckpointError::StaleCapture)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{
        EventSeq, SessionId, WorktreeId,
        checkpoint::{Boundary, CHECKPOINT_QUEUE, CaptureGeneration, CaptureId},
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
    #[tokio::test]
    async fn actor_can_continue_after_a_typed_unavailable_acknowledgement() {
        let (sender, mut received) = mpsc::channel::<CaptureBoundary>(CHECKPOINT_QUEUE);
        let key = key();
        let node = tokio::spawn(async move {
            let request = received.recv().await.unwrap();
            assert_eq!(request.key, key);
            request
                .reply
                .send(Err(CheckpointError::CaptureTimedOut))
                .unwrap();
        });
        assert_eq!(
            acknowledge(&sender, key).await,
            Err(CheckpointError::CaptureTimedOut)
        );
        node.await.unwrap();
    }
    #[test]
    #[ignore = "skeleton: connect this seam to real prompt delivery and durable TurnEnded"]
    fn queued_prompts_capture_at_delivery_but_steers_do_not_create_boundaries() {
        // Proof obligation: implement real actor-boundary delivery fixture.
    }
}
