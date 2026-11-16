//! The queue's side of the actor (protocol 10, ADR 0024): a
//! `session.queue` change carried out, the queue moving or held after a
//! turn's end and at the session's end, and published when it changed.

use vessel_proto::conversation::QueueChanged;
use vessel_proto::{Delivered, Delivery, QueueChange, QueueHold, QueuedPrompt};

use super::submission::{SubmissionDispatch, known_or_uncertain};
use super::{Change, TurnEnd, TurnTicket};
use crate::error::{DeliveryError, SessionError};
use crate::queue::{Applied, Queue};
use crate::structured::StructuredActor;

enum EntryFailure {
    Restored,
    Uncertain,
}

impl StructuredActor {
    /// Carry out a `session.queue` change and answer it.
    pub(crate) async fn change_queue(&mut self, order: Change) {
        let result = self.change(order.change).await;
        let _gone = order.reply.send(result);
    }

    /// After a turn's end was published: the queue moves, or is held.
    pub(crate) async fn after_turn(&mut self) {
        let Some(end) = self.desk.ended.take() else {
            return;
        };
        match end {
            TurnEnd::Done => {
                let _started = self.advance().await;
            }
            TurnEnd::Cancelled => {
                if self.desk.queue.hold(QueueHold::Cancelled) {
                    self.publish_queue().await;
                }
            }
            TurnEnd::Failed => {
                if self.desk.queue.hold(QueueHold::Failed) {
                    self.publish_queue().await;
                }
            }
        }
    }

    /// The session ends: what waits is kept, held (`ended`).
    pub(crate) async fn queue_at_end(&mut self) {
        if self.desk.queue.end() {
            self.publish_queue().await;
        }
    }

    async fn change(&mut self, change: QueueChange) -> Result<QueueChanged, SessionError> {
        if let QueueChange::Edit { content, .. } = &change {
            self.check_attachments(content)?;
        }
        let delivered = match self.desk.queue.apply(change)? {
            Applied::Changed => None,
            Applied::Resumed => {
                self.publish_queue().await;
                self.advance().await
            }
            Applied::Taken {
                entry,
                place,
                delivery,
            } => Some(self.send_taken(entry, place, delivery).await?),
        };
        self.publish_queue().await;
        Ok(QueueChanged {
            queue: self.desk.queue.get().clone(),
            delivered,
        })
    }

    async fn send_taken(
        &mut self,
        entry: QueuedPrompt,
        place: usize,
        delivery: Delivery,
    ) -> Result<Delivered, SessionError> {
        if let Some(ticket) = self.desk.tickets.take(entry.id)
            && let Err(ticket) = self.desk.tickets.prepare(ticket)
        {
            self.restore_entry(place, entry, Some(ticket));
            return Err(DeliveryError::ReceiptCapacity.into());
        }
        let sent = self
            .send_as(
                entry.content.clone(),
                delivery,
                entry.by.clone(),
                entry.client_message_id.clone(),
                entry.provenance,
            )
            .await;
        match sent {
            Ok(delivered) => {
                self.desk.queue.settle();
                let started = self.desk.started();
                self.desk.tickets.finish_prepared(delivered, started);
                Ok(delivered)
            }
            Err(error) => {
                let _state = self.failed_entry(place, entry, &error);
                self.publish_queue().await;
                Err(error)
            }
        }
    }

    fn restore_entry(&mut self, place: usize, entry: QueuedPrompt, ticket: Option<TurnTicket>) {
        let id = entry.id;
        if let Err(full) = self.desk.queue.insert(place, entry) {
            tracing::warn!(%full, "a prompt not sent could not go back to the queue");
        } else if let Some(ticket) = ticket {
            self.desk.tickets.keep(id, ticket);
        }
    }

    fn failed_entry(
        &mut self,
        place: usize,
        entry: QueuedPrompt,
        error: &SessionError,
    ) -> EntryFailure {
        let ticket = self.desk.tickets.take_prepared();
        if ticket.as_ref().is_some_and(TurnTicket::is_receipt)
            && !matches!(known_or_uncertain(error), SubmissionDispatch::Refused(_))
        {
            // A possibly-sent canonical entry must not become actionable again
            // in this actor. Durable pre-dispatch claims remain a separate gate.
            if let Some(ticket) = ticket {
                self.desk.tickets.keep_uncertain(ticket);
            }
            EntryFailure::Uncertain
        } else {
            self.restore_entry(place, entry, ticket);
            EntryFailure::Restored
        }
    }

    /// Start the queue's first entry, when it moves and nothing runs.
    async fn advance(&mut self) -> Option<Delivered> {
        if self.desk.running() || !self.ready {
            return None;
        }
        let entry = self.desk.queue.next()?;
        if let Some(ticket) = self.desk.tickets.take(entry.id)
            && let Err(ticket) = self.desk.tickets.prepare(ticket)
        {
            self.restore_entry(0, entry, Some(ticket));
            let _held = self.desk.queue.hold(QueueHold::Failed);
            self.publish_queue().await;
            return None;
        }
        match self
            .start_admitted(
                entry.content.clone(),
                entry.client_message_id.clone(),
                entry.provenance,
            )
            .await
        {
            Ok(()) => {
                self.desk.queue.settle();
                let started = self.desk.started();
                self.desk
                    .tickets
                    .finish_prepared(Delivered::Started, started);
                self.publish_queue().await;
                Some(Delivered::Started)
            }
            Err(error) => {
                let line = match self.failed_entry(0, entry, &error) {
                    EntryFailure::Restored => format!(
                        "vessel: the queue's next prompt was not sent ({error}); the queue is held"
                    ),
                    EntryFailure::Uncertain => "vessel: the queued delivery is uncertain; its entry is protected from resend".to_owned(),
                };
                let _held = self.desk.queue.hold(QueueHold::Failed);
                self.publish_queue().await;
                self.life.notices(&mut self.agent, vec![line]).await;
                None
            }
        }
    }

    /// Publish the queue if it differs from what was last published.
    pub(crate) async fn publish_queue(&mut self) {
        if self.desk.published.get() == self.desk.queue.get() {
            self.desk
                .tickets
                .prune(self.desk.queue.get(), self.life.out.next_seq());
            return;
        }
        let queue = self.desk.queue.get().clone();
        self.desk.published = Queue::from(queue.clone());
        self.life
            .publish(
                &mut self.agent,
                vessel_proto::SessionEvent::QueueChanged { queue },
            )
            .await;
        // Receipt withdrawal cannot precede its canonical queue removal. Legacy
        // tickets still close; a lost owner alone never emits a no-send fact.
        self.desk
            .tickets
            .prune(self.desk.queue.get(), self.life.out.next_seq());
    }
}
