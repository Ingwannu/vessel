//! Actual turn boundaries: queued delivery waits; injected steers create no anchor.
use crate::{CheckpointAdvance, CheckpointReceipt, SessionError, structured::StructuredActor};
use tokio::time::Instant;
use vessel_agent::InputError;
use vessel_proto::{
    SessionEvent,
    checkpoint::{Boundary, CHECKPOINT_CAPTURE_DEADLINE, CheckpointEvent},
};
impl StructuredActor {
    pub(crate) async fn before_checkpoint(
        &mut self,
    ) -> Result<Option<CheckpointReceipt>, SessionError> {
        let Some(hooks) = self.checkpoints.clone() else {
            return Ok(None);
        };
        // The driver's ordered query also waits for initial readiness. A prompt
        // refused as already-running has no new file checkpoint boundary.
        let input = self.input.clone();
        let running = self
            .pump(async move { input.turn_running().await })
            .await
            .map_err(|_stopped| SessionError::Ended)??;
        if running {
            return Err(InputError::TurnInProgress.into());
        }
        self.checkpoint_idle().await?;
        // A completed turn can arrive while a delivery is being pumped. Capture
        // its after-image before this new prompt, without advancing another queue.
        self.after_checkpoint().await;
        let deadline = Instant::now()
            .checked_add(CHECKPOINT_CAPTURE_DEADLINE)
            .ok_or(SessionError::CheckpointBusy)?;
        let Some(prompt) = self.checkpoint_anchor(deadline).await? else {
            return Ok(None);
        };
        let session = self.life.out.session();
        let receipt = self
            .pump(async move {
                hooks
                    .capture_by(session, Boundary::BeforeTurn { prompt }, deadline)
                    .await
            })
            .await
            .map_err(|_stopped| SessionError::Ended)?
            .ok();
        self.checkpoint_prompt = Some(prompt);
        Ok(receipt)
    }
    async fn checkpoint_anchor(
        &mut self,
        deadline: Instant,
    ) -> Result<Option<vessel_proto::EventSeq>, SessionError> {
        let prompt = self.life.out.next_seq();
        let _published = tokio::time::timeout_at(
            deadline,
            self.life.publish(
                &mut self.agent,
                SessionEvent::Checkpoint {
                    event: CheckpointEvent::PromptAccepted,
                },
            ),
        )
        .await;
        if Some(self.life.out.next_seq()) != prompt.next() {
            // A congested durable channel cannot spend an unbounded capture
            // budget. No accepted anchor means no restorable before-image.
            return if self.life.stop.killed() {
                Err(SessionError::Ended)
            } else {
                Ok(None)
            };
        }
        Ok(Some(prompt))
    }
    /// Driver command completion can win the select before its already-buffered
    /// Ready/TurnEnded output. Settle those observations before naming an anchor.
    async fn checkpoint_idle(&mut self) -> Result<(), SessionError> {
        let deadline = Instant::now()
            .checked_add(CHECKPOINT_CAPTURE_DEADLINE)
            .ok_or(SessionError::CheckpointBusy)?;
        while !self.ready || self.desk.running() {
            if Instant::now() >= deadline {
                return Err(SessionError::CheckpointBusy);
            }
            tokio::select! {
                biased;
                step = self.life.stop.next() => { self.life.apply(&mut self.agent, step); return Err(SessionError::Ended); }
                output = self.outputs.next(), if self.outputs_open => if let Some(output) = output {
                    self.output(output).await;
                } else {
                    self.outputs_open = false;
                    return Err(SessionError::Ended);
                },
                () = tokio::time::sleep_until(deadline) => return Err(SessionError::CheckpointBusy),
            }
        }
        Ok(())
    }
    pub(crate) fn checkpoint_sent(
        &mut self,
        receipt: Option<CheckpointReceipt>,
        sent: Result<(), SessionError>,
    ) -> Result<(), SessionError> {
        let advance = if sent.is_ok() {
            CheckpointAdvance::PromptSent
        } else {
            self.checkpoint_prompt = None;
            CheckpointAdvance::PromptRefused
        };
        if let Some(receipt) = receipt {
            receipt.complete(advance);
        }
        sent
    }
    pub(crate) async fn after_checkpoint(&mut self) {
        let Some(ended) = self.desk.checkpoint_ended.take() else {
            return;
        };
        let Some(hooks) = self.checkpoints.clone() else {
            return;
        };
        let session = self.life.out.session();
        let captured = self
            .pump(async move { hooks.capture(session, Boundary::AfterTurn { ended }).await })
            .await;
        if let Ok(Ok(receipt)) = captured {
            receipt.complete(CheckpointAdvance::TurnComplete);
        }
    }
}

pub(crate) struct TurnAssociation {
    prompt: vessel_proto::EventSeq,
    turn: vessel_proto::EventSeq,
}
impl StructuredActor {
    pub(crate) fn checkpoint_association(
        &mut self,
        event: &vessel_proto::AgentEvent,
    ) -> Option<TurnAssociation> {
        if matches!(event, vessel_proto::AgentEvent::TurnStarted { .. }) {
            self.checkpoint_prompt.take().map(|prompt| TurnAssociation {
                prompt,
                turn: self.life.out.next_seq(),
            })
        } else {
            None
        }
    }
    pub(crate) async fn publish_association(&mut self, association: Option<TurnAssociation>) {
        if let Some(TurnAssociation { prompt, turn }) = association {
            self.life
                .publish(
                    &mut self.agent,
                    SessionEvent::Checkpoint {
                        event: CheckpointEvent::TurnAssociated { prompt, turn },
                    },
                )
                .await;
        }
    }
}
