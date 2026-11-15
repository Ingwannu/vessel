//! Only the owned pause can ask its actor to publish restore-result metadata.
use crate::{SessionError, structured::StructuredActor};
use tokio::sync::oneshot;
use vessel_proto::{
    EventSeq, SessionEvent,
    checkpoint::{CheckpointEvent, CheckpointId, PathCount, RestoreId},
};

#[derive(Debug)]
pub(crate) struct Order {
    event: CheckpointEvent,
    done: oneshot::Sender<Result<EventSeq, SessionError>>,
}

impl super::CheckpointPause {
    /// Publish a verified restore result while keeping this pause held.
    ///
    /// The returned sequence identifies actor publication, not SQL durability.
    /// The node must await the store's matching restore-row event marker before
    /// releasing physical exclusion and resuming the actor. Identical retries
    /// on this pause reuse the sequence; a different result is refused.
    ///
    /// # Errors
    /// The actor ended, released this pause, or already published another result.
    pub async fn files_restored(
        &mut self,
        restore: RestoreId,
        checkpoint: CheckpointId,
        paths: PathCount,
    ) -> Result<EventSeq, SessionError> {
        let (done, reply) = oneshot::channel();
        self.metadata
            .send(Order {
                event: CheckpointEvent::FilesRestored {
                    restore,
                    checkpoint,
                    paths,
                },
                done,
            })
            .await
            .map_err(|_ended| SessionError::Ended)?;
        reply.await.map_err(|_ended| SessionError::Ended)?
    }
}

impl StructuredActor {
    pub(crate) async fn checkpoint_metadata(&mut self, order: Order) {
        match self.file_pause.publication() {
            super::pause::Publication::Released => {
                let _gone = order.done.send(Err(SessionError::FilesPaused));
                return;
            }
            super::pause::Publication::Published { event, seq } => {
                let result = if event == &order.event {
                    Ok(seq)
                } else {
                    Err(SessionError::CheckpointBusy)
                };
                let _gone = order.done.send(result);
                return;
            }
            super::pause::Publication::Pending => {}
        }
        let seq = self.life.out.next_seq();
        self.life
            .publish(
                &mut self.agent,
                SessionEvent::Checkpoint {
                    event: order.event.clone(),
                },
            )
            .await;
        if Some(self.life.out.next_seq()) != seq.next() {
            let _gone = order.done.send(Err(SessionError::Ended));
            return;
        }
        self.file_pause.record(order.event, seq);
        let _gone = order.done.send(Ok(seq));
    }
}
