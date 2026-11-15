//! The queue's side of the actor (protocol 10, ADR 0024): a
//! `session.queue` change carried out, the queue moving or held after a
//! turn's end and at the session's end, and published when it changed.

use vessel_proto::conversation::QueueChanged;
use vessel_proto::{Delivered, QueueChange, QueueHold};

use super::{Change, TurnEnd};
use crate::error::SessionError;
use crate::queue::{Applied, Queue};
use crate::structured::StructuredActor;

impl StructuredActor {
    /// Carry out a `session.queue` change and answer it.
    pub(crate) async fn change_queue(&mut self, order: Change) {
        self.desk.admission = order.generation;
        let result = self.change(order.action).await;
        let _gone = order.reply.send(result);
    }

    /// After a turn's end was published: the queue moves, or is held.
    pub(crate) async fn after_turn(&mut self) {
        let Some(end) = self.desk.ended.take() else {
            return;
        };
        self.desk.admission = self.desk.accepted;
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
            } => {
                let id = entry.id;
                let ticket = self.desk.tickets.take(id);
                let sent = self
                    .send_as(entry.content.clone(), delivery, entry.by.clone())
                    .await;
                match sent {
                    Ok(delivered) => {
                        self.desk.queue.settle();
                        if let Some(ticket) = ticket {
                            let started = self.desk.started();
                            self.desk.tickets.give(ticket, delivered, started);
                        }
                        Some(delivered)
                    }
                    Err(error) => {
                        // Back where it was; nothing was sent.
                        if let Err(full) = self.desk.queue.insert(place, entry) {
                            tracing::warn!(%full, "a prompt not sent could not go back to the queue");
                        } else if let Some(ticket) = ticket {
                            self.desk.tickets.keep(id, ticket);
                        }
                        return Err(error);
                    }
                }
            }
        };
        self.publish_queue().await;
        Ok(QueueChanged {
            queue: self.desk.queue.get().clone(),
            delivered,
        })
    }

    /// Start the queue's first entry, when it moves and nothing runs.
    async fn advance(&mut self) -> Option<Delivered> {
        if self.desk.running() || !self.ready {
            return None;
        }
        let entry = self.desk.queue.next()?;
        let id = entry.id;
        let ticket = self.desk.tickets.take(id);
        match self.start(entry.content.clone()).await {
            Ok(()) => {
                self.desk.queue.settle();
                if let Some(ticket) = ticket {
                    let started = self.desk.started();
                    self.desk.tickets.give(ticket, Delivered::Started, started);
                }
                self.publish_queue().await;
                Some(Delivered::Started)
            }
            Err(error) => {
                // It goes back first, the queue held: the agent may take
                // less than when it was queued (after a move), or it is
                // going away.
                let line = format!(
                    "vessel: the queue's next prompt was not sent ({error}); the queue is held"
                );
                if let Err(full) = self.desk.queue.insert(0, entry) {
                    tracing::warn!(%full, "a prompt not sent could not go back to the queue");
                } else if let Some(ticket) = ticket {
                    self.desk.tickets.keep(id, ticket);
                }
                let _held = self.desk.queue.hold(QueueHold::Failed);
                self.publish_queue().await;
                self.life.notices(&mut self.agent, vec![line]).await;
                None
            }
        }
    }

    /// Publish the queue if it differs from what was last published.
    pub(crate) async fn publish_queue(&mut self) {
        // An entry that left the queue without a turn starts none.
        self.desk.tickets.prune(self.desk.queue.get());
        if self.desk.published.get() == self.desk.queue.get() {
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
    }
}
