//! Boundary acknowledgement has one deadline across every bounded queue hop.
use tokio::{
    sync::{mpsc, oneshot},
    time::Instant,
};
use vessel_proto::{
    SessionId,
    checkpoint::{
        Boundary, CHECKPOINT_CAPTURE_DEADLINE, CHECKPOINT_QUEUE, CaptureKey, CheckpointError,
    },
};

/// How the actor advanced past an immutable capture boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointAdvance {
    /// The provider accepted delivery; the node may release its prompt hold.
    PromptSent,
    /// Delivery failed before acceptance.
    PromptRefused,
    /// The after-turn boundary completed before queue advancement.
    TurnComplete,
}
/// A bounded bridge to the node's filesystem/storage owner.
#[derive(Debug, Clone)]
pub struct CheckpointHooks {
    requests: mpsc::Sender<CheckpointRequest>,
}
/// One immutable actor boundary; only the node supplies physical scope/generation.
#[derive(Debug)]
pub struct CheckpointRequest {
    session: SessionId,
    boundary: Boundary,
    deadline: Instant,
    reply: oneshot::Sender<CheckpointReceipt>,
}
/// A node acknowledgement, plus an actor receipt for its prompt hold.
#[derive(Debug)]
pub struct CheckpointReceipt {
    result: Result<CaptureKey, CheckpointError>,
    delivered: oneshot::Sender<CheckpointAdvance>,
}
impl CheckpointReceipt {
    /// The capture's exact association or typed unavailability.
    /// # Errors
    /// The node refused or could not capture this exact boundary before deadline.
    pub const fn result(&self) -> Result<CaptureKey, CheckpointError> {
        self.result
    }
    /// Complete after provider delivery settles, or the after-turn boundary completes.
    /// Dropping without an answer is uncertain advancement, never success.
    pub fn complete(self, outcome: CheckpointAdvance) {
        let _gone = self.delivered.send(outcome);
    }
}
impl CheckpointRequest {
    /// Session whose durable accepted-prompt or turn-ended record anchors this request.
    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.session
    }
    /// Immutable durable boundary, never a provider string or guessed future sequence.
    #[must_use]
    pub const fn boundary(&self) -> Boundary {
        self.boundary
    }
    /// The original actor deadline; node queues must preserve it.
    #[must_use]
    pub const fn deadline(&self) -> Instant {
        self.deadline
    }
    /// Refuse admission before reserving resources for a timed-out/abandoned actor.
    #[must_use]
    pub fn abandoned(&self) -> bool {
        self.reply.is_closed() || Instant::now() >= self.deadline
    }
    /// Acknowledge capture without transferring any filesystem authority.
    /// The returned receiver lets the node retain prompt exclusion through delivery.
    /// A closed receiver means delivery is uncertain, not a successful send.
    #[must_use = "retain delivery exclusion until the actor reports provider acceptance or refusal"]
    pub fn acknowledge(
        self,
        result: Result<CaptureKey, CheckpointError>,
    ) -> oneshot::Receiver<CheckpointAdvance> {
        let result = result.and_then(|key| {
            if key.session == self.session && key.boundary == self.boundary {
                Ok(key)
            } else {
                Err(CheckpointError::StaleCapture)
            }
        });
        let (delivered, receipt) = oneshot::channel();
        let _gone = self.reply.send(CheckpointReceipt { result, delivered });
        receipt
    }
}
impl CheckpointHooks {
    /// Construct the bounded actor bridge. A full queue immediately returns Busy.
    #[must_use]
    pub fn channel() -> (Self, mpsc::Receiver<CheckpointRequest>) {
        let (requests, received) = mpsc::channel(CHECKPOINT_QUEUE);
        (Self { requests }, received)
    }
    /// Ask for a boundary, bounding acknowledgement including queue time.
    ///
    /// # Errors
    /// Busy/closed queue, deadline, or a lost node acknowledgement. These make
    /// capture unavailable; the actor may continue its normal prompt lifecycle.
    pub async fn capture(
        &self,
        session: SessionId,
        boundary: Boundary,
    ) -> Result<CheckpointReceipt, CheckpointError> {
        let deadline = Instant::now()
            .checked_add(CHECKPOINT_CAPTURE_DEADLINE)
            .ok_or(CheckpointError::CaptureTimedOut)?;
        self.capture_by(session, boundary, deadline).await
    }
    /// Preserve the deadline when the actor first publishes its durable anchor.
    ///
    /// # Errors
    /// The same queue/acknowledgement failures as `capture`; an expired deadline
    /// refuses before queueing and cannot acquire a fresh timeout budget.
    pub async fn capture_by(
        &self,
        session: SessionId,
        boundary: Boundary,
        deadline: Instant,
    ) -> Result<CheckpointReceipt, CheckpointError> {
        if Instant::now() >= deadline {
            return Err(CheckpointError::CaptureTimedOut);
        }
        let (reply, received) = oneshot::channel();
        self.requests
            .try_send(CheckpointRequest {
                session,
                boundary,
                deadline,
                reply,
            })
            .map_err(|_full_or_closed| CheckpointError::Busy)?;
        let receipt = tokio::time::timeout_at(deadline, received)
            .await
            .map_err(|_deadline| CheckpointError::CaptureTimedOut)?
            .map_err(|_closed| CheckpointError::StaleCapture)?;
        // A ready reply and an expired timer may be polled together. The clock,
        // not select scheduling, decides whether the actor may accept the reply.
        if Instant::now() >= deadline {
            return Err(CheckpointError::CaptureTimedOut);
        }
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{
        EventSeq, WorktreeId,
        checkpoint::{CaptureGeneration, CaptureId},
    };
    fn key(session: SessionId, boundary: Boundary) -> CaptureKey {
        CaptureKey {
            capture: CaptureId::random(),
            session,
            worktree: WorktreeId::random(),
            boundary,
            generation: CaptureGeneration::FIRST,
        }
    }
    #[tokio::test]
    async fn checkpoint_hook_unavailable_receipt_still_tracks_normal_actor_delivery() {
        let (hooks, mut node) = CheckpointHooks::channel();
        let session = SessionId::random();
        let boundary = Boundary::BeforeTurn {
            prompt: EventSeq::new(4),
        };
        let actor = tokio::spawn(async move { hooks.capture(session, boundary).await });
        let request = node.recv().await.unwrap();
        assert_eq!(request.session(), session);
        assert_eq!(request.boundary(), boundary);
        assert!(!request.abandoned());
        let delivered = request.acknowledge(Err(CheckpointError::StorageFull));
        let receipt = actor.await.unwrap().unwrap();
        assert_eq!(receipt.result(), Err(CheckpointError::StorageFull));
        receipt.complete(CheckpointAdvance::PromptSent);
        assert_eq!(delivered.await.unwrap(), CheckpointAdvance::PromptSent);
    }
    #[tokio::test(start_paused = true)]
    async fn checkpoint_hook_deadline_releases_actor_and_late_ack_cannot_claim_delivery() {
        let (hooks, mut node) = CheckpointHooks::channel();
        let session = SessionId::random();
        let boundary = Boundary::AfterTurn {
            ended: EventSeq::new(8),
        };
        let actor = tokio::spawn(async move { hooks.capture(session, boundary).await });
        let request = node.recv().await.unwrap();
        let deadline = request.deadline();
        tokio::time::advance(CHECKPOINT_CAPTURE_DEADLINE).await;
        assert!(matches!(
            actor.await.unwrap(),
            Err(CheckpointError::CaptureTimedOut)
        ));
        assert!(request.abandoned());
        assert_eq!(deadline, Instant::now());
        let late = request.acknowledge(Ok(key(session, boundary)));
        assert!(late.await.is_err());
    }
    #[tokio::test]
    async fn checkpoint_hook_refuses_wrong_associations_and_abandoned_delivery_is_uncertain() {
        let (hooks, mut node) = CheckpointHooks::channel();
        let session = SessionId::random();
        let boundary = Boundary::BeforeTurn {
            prompt: EventSeq::new(4),
        };
        let actor = tokio::spawn(async move { hooks.capture(session, boundary).await });
        let request = node.recv().await.unwrap();
        let delivered = request.acknowledge(Ok(key(SessionId::random(), boundary)));
        let receipt = actor.await.unwrap().unwrap();
        assert_eq!(receipt.result(), Err(CheckpointError::StaleCapture));
        drop(receipt);
        assert!(delivered.await.is_err());
    }
}

#[cfg(test)]
mod obligations {
    #[test]
    #[ignore = "skeleton: real node capture must await committed TurnEnded before queue advance"]
    fn queued_prompts_capture_at_delivery_but_steers_do_not_create_boundaries() {
        // Actor delivery is connected; real node storage/capture ordering is still owed.
    }
}
