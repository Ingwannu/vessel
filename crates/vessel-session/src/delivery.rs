//! Prompts as they are delivered, and the queue moving (protocol 10, ADR
//! 0024), carried out by the structured session's actor.
//!
//! [Decision Log]
//! - Purpose: `session.prompt`'s four deliveries and the node's queue, one
//!   at a time, so an interrupt waiting for its cancel is not overtaken by
//!   the queue's next entry and a steer sent meanwhile waits its turn
//!   (the contract's review, item 7).
//! - Choice: the actor carries them out as it carries out a move
//!   (`moving`): orders come through its control channel, and whatever it
//!   awaits from the agent it awaits while still publishing the agent's
//!   outputs (`pump`), so an agent that writes a lot cannot deadlock with
//!   a delivery. Prompts used to go from a handle straight to the agent;
//!   they now take the actor's turn, one hop more.
//! - Alternatives: a task of its own per session keeping the queue and
//!   asking the actor to publish (two owners of one state, and every
//!   turn's end passed across); the agent's driver keeping it (it would
//!   decide records the session's log orders).
//! - The queue moves only from the actor's loop, after a turn's end was
//!   published (`after_turn`), never inside the output that published it,
//!   so starting the next entry is a delivery like any other.
//! - What the session knows of turns is what it published: a turn runs
//!   from the prompt it sent until the turn's end is in the log. When the
//!   agent ended a turn whose end is still on its way, a steer is handed
//!   back (`promptRequired`) and starts a turn, and an interrupt finds the
//!   end as it waits; neither is lost.

use tokio::sync::oneshot;
use vessel_agent::{InputError, SteerOutcome};
use vessel_proto::conversation::QueueChanged;
use vessel_proto::{
    AgentAbilities, AgentEvent, ContentBlock, Creator, Delivered, Delivery, EventSeq, PromptQueue,
    QueueChange, QueueHold, StopReason,
};

use crate::error::{DeliveryError, SessionError};
use crate::queue::{Applied, Queue};
use crate::structured::StructuredActor;

/// `session.prompt` (and `session.send`'s prompt, which is `start`).
#[derive(Debug)]
pub(crate) struct Deliver {
    pub(crate) content: Vec<ContentBlock>,
    pub(crate) delivery: Delivery,
    pub(crate) by: Creator,
    pub(crate) reply: oneshot::Sender<Result<Delivered, SessionError>>,
}

/// `session.queue`.
#[derive(Debug)]
pub(crate) struct Change {
    pub(crate) change: QueueChange,
    pub(crate) reply: oneshot::Sender<Result<QueueChanged, SessionError>>,
}

/// `session.send`'s `cancel_turn`.
#[derive(Debug)]
pub(crate) struct Cancel {
    /// The turn meant (its `turn_started` record); `None` is whichever
    /// runs.
    pub(crate) turn: Option<EventSeq>,
    pub(crate) reply: oneshot::Sender<Result<(), SessionError>>,
}

/// How a turn ended, for the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TurnEnd {
    Done,
    Cancelled,
    Failed,
}

/// What the actor keeps for deliveries.
#[derive(Debug, Default)]
pub(crate) struct Desk {
    pub(crate) queue: Queue,
    /// The queue as last published, so a change that changes nothing logs
    /// nothing.
    published: Queue,
    /// The running turn's `turn_started` record, once published.
    turn: Option<EventSeq>,
    /// Turns the actor started, and turns whose end it published. Every
    /// prompt goes through the actor, so a turn runs while the first is
    /// ahead; counted rather than flagged because a turn's end can be
    /// published while the prompt's own answer is still on its way.
    started: u64,
    ended_turns: u64,
    /// A turn's end the queue has not acted on yet.
    ended: Option<TurnEnd>,
    /// What the agent said it can do, from its `ready`.
    abilities: AgentAbilities,
    /// An interrupt waits for the turn it cancelled; that turn's end is
    /// the interrupt's, not the queue's.
    interrupting: bool,
}

impl Desk {
    /// A session started again (protocol 10, ADR 0026): the queue its
    /// stored log left comes back held `ended` when anything waits, as the
    /// session's end leaves it (also when the node stopped before the end
    /// could say so). `published` is the log's, so the actor publishes the
    /// queue again only if this changed it.
    pub(crate) fn continuing(queue: PromptQueue) -> Self {
        let published = Queue::from(queue.clone());
        let mut queue = Queue::from(queue);
        let _held = queue.end();
        Self {
            queue,
            published,
            ..Self::default()
        }
    }

    /// Take in an agent event about to be published as record `seq`.
    pub(crate) fn observe(&mut self, event: &AgentEvent, seq: EventSeq) {
        match event {
            AgentEvent::Ready { abilities, .. } => self.ready(*abilities),
            AgentEvent::TurnStarted { .. } => self.turn = Some(seq),
            AgentEvent::TurnEnded { reason, .. } => {
                let end = if *reason == StopReason::Cancelled {
                    TurnEnd::Cancelled
                } else {
                    TurnEnd::Done
                };
                self.over(end);
            }
            AgentEvent::TurnFailed { .. } => self.over(TurnEnd::Failed),
            AgentEvent::UserMessageChunk { .. }
            | AgentEvent::MessageChunk { .. }
            | AgentEvent::ThoughtChunk { .. }
            | AgentEvent::ToolCall(_)
            | AgentEvent::ToolCallUpdate(_)
            | AgentEvent::Plan { .. }
            | AgentEvent::PermissionRequested(_)
            | AgentEvent::PermissionResolved { .. }
            | AgentEvent::HarnessLog { .. }
            | AgentEvent::Steered { .. }
            | AgentEvent::QuestionAsked(_)
            | AgentEvent::QuestionResolved { .. } => {}
        }
    }

    /// The agent is ready (also one a move started, whose `ready` the log
    /// does not get again).
    pub(crate) fn ready(&mut self, abilities: Option<AgentAbilities>) {
        if let Some(abilities) = abilities {
            self.abilities = abilities;
        }
    }

    /// Whether a turn runs.
    pub(crate) const fn running(&self) -> bool {
        self.started > self.ended_turns
    }

    fn over(&mut self, end: TurnEnd) {
        self.turn = None;
        self.ended_turns = self.ended_turns.saturating_add(1);
        if !self.interrupting {
            self.ended = Some(end);
        }
    }
}

impl StructuredActor {
    /// Carry out a `session.prompt` and answer it.
    pub(crate) async fn deliver(&mut self, order: Deliver) {
        let Deliver {
            content,
            delivery,
            by,
            reply,
        } = order;
        let result = self.send_as(content, delivery, by).await;
        let _gone = reply.send(result);
    }

    /// Carry out a `session.queue` change and answer it.
    pub(crate) async fn change_queue(&mut self, order: Change) {
        let result = self.change(order.change).await;
        let _gone = order.reply.send(result);
    }

    /// A person's cancel: the queue is held (nothing waiting is sent after
    /// a stop), and the turn named, if it still runs, is cancelled.
    pub(crate) async fn cancel(&mut self, order: Cancel) {
        let Cancel { turn, reply } = order;
        if self.desk.queue.hold(QueueHold::Cancelled) {
            self.publish_queue().await;
        }
        let result = if turn.is_none() || turn == self.desk.turn {
            let input = self.input.clone();
            self.pump(async move { input.cancel_turn().await })
                .await
                .map_err(|_stopped| SessionError::Ended)
                .and_then(|cancelled| cancelled.map_err(SessionError::from))
        } else {
            // That turn ended and another runs: it goes on.
            Ok(())
        };
        let _gone = reply.send(result);
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

    async fn send_as(
        &mut self,
        content: Vec<ContentBlock>,
        delivery: Delivery,
        by: Creator,
    ) -> Result<Delivered, SessionError> {
        match delivery {
            Delivery::Start => self.start(content).await.map(|()| Delivered::Started),
            Delivery::Queue => {
                let idle = !self.desk.running() && self.desk.queue.is_empty();
                if idle && !self.desk.queue.is_held() {
                    return self.start(content).await.map(|()| Delivered::Started);
                }
                let entry = self.desk.queue.push(content, by)?;
                self.publish_queue().await;
                Ok(Delivered::Queued { entry })
            }
            Delivery::Steer => self.steer(content, by).await,
            Delivery::Interrupt => self.interrupt(content).await,
        }
    }

    async fn start(&mut self, content: Vec<ContentBlock>) -> Result<(), SessionError> {
        let input = self.input.clone();
        self.pump(async move { input.prompt(content).await })
            .await
            .map_err(|_stopped| SessionError::Ended)??;
        self.desk.started = self.desk.started.saturating_add(1);
        Ok(())
    }

    async fn steer(
        &mut self,
        content: Vec<ContentBlock>,
        by: Creator,
    ) -> Result<Delivered, SessionError> {
        // With no turn running a steer starts one, whatever the agent can
        // do (also while it starts and has not said yet).
        if !self.desk.running() {
            return self.start(content).await.map(|()| Delivered::Started);
        }
        if !self.desk.abilities.steer {
            return Err(InputError::CannotSteer.into());
        }
        let input = self.input.clone();
        let sent = content.clone();
        let outcome = self
            .pump(async move { input.steer(sent).await })
            .await
            .map_err(|_stopped| SessionError::Ended)??;
        match outcome {
            SteerOutcome::Injected => Ok(Delivered::Steered),
            // The turn ended first: the message starts the next one, or,
            // if one started meanwhile, goes first in the queue.
            SteerOutcome::PromptRequired => match self.start(content.clone()).await {
                Ok(()) => Ok(Delivered::Started),
                Err(SessionError::Input(InputError::TurnInProgress)) => {
                    let entry = self.desk.queue.push_first(content, by)?;
                    self.publish_queue().await;
                    Ok(Delivered::Queued { entry })
                }
                Err(error) => Err(error),
            },
        }
    }

    /// Cancel the running turn, wait (bounded) for its end, then start
    /// `content`. The queue is not held by this cancel.
    async fn interrupt(&mut self, content: Vec<ContentBlock>) -> Result<Delivered, SessionError> {
        if self.desk.running() {
            self.desk.interrupting = true;
            let over = self.cancel_and_wait().await;
            self.desk.interrupting = false;
            if !over? {
                return Err(DeliveryError::TurnDidNotEnd.into());
            }
        }
        self.start(content).await.map(|()| Delivered::Started)
    }

    /// Whether the running turn ended after a cancel.
    async fn cancel_and_wait(&mut self) -> Result<bool, SessionError> {
        self.move_state.turn_over = false;
        let input = self.input.clone();
        self.pump(async move { input.cancel_turn().await })
            .await
            .map_err(|_stopped| SessionError::Ended)??;
        if !self.desk.running() {
            return Ok(true);
        }
        self.until_turn_over()
            .await
            .map_err(|_stopped| SessionError::Ended)
    }

    async fn change(&mut self, change: QueueChange) -> Result<QueueChanged, SessionError> {
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
                let sent = self
                    .send_as(entry.content.clone(), delivery, entry.by.clone())
                    .await;
                match sent {
                    Ok(delivered) => {
                        self.desk.queue.settle();
                        Some(delivered)
                    }
                    Err(error) => {
                        // Back where it was; nothing was sent.
                        if let Err(full) = self.desk.queue.insert(place, entry) {
                            tracing::warn!(%full, "a prompt not sent could not go back to the queue");
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
        match self.start(entry.content.clone()).await {
            Ok(()) => {
                self.desk.queue.settle();
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
