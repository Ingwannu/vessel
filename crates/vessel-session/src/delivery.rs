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
//!
//! Layout: this module carries out deliveries and cancels; `moves` the
//! queue's changes and its moving after a turn.

mod moves;
mod tickets;

pub(crate) use tickets::TurnTicket;

use tokio::sync::oneshot;
use vessel_agent::{Attachments, InputError, SteerOutcome};
use vessel_proto::conversation::QueueChanged;
use vessel_proto::{
    AgentAbilities, AgentEvent, ContentBlock, Creator, Delivered, Delivery, EventSeq, PromptQueue,
    QueueChange, QueueHold, StopReason,
};

use crate::error::{DeliveryError, SessionError};
use crate::queue::Queue;
use crate::structured::StructuredActor;

/// `session.prompt` (and `session.send`'s prompt, which is `start`).
#[derive(Debug)]
pub(crate) struct Deliver {
    pub(crate) generation: vessel_proto::restart::RestartGeneration,
    pub(crate) content: Vec<ContentBlock>,
    pub(crate) delivery: Delivery,
    pub(crate) by: Creator,
    /// Told which turn it started or joined (protocol 11, `tickets`).
    pub(crate) turn: Option<TurnTicket>,
    pub(crate) reply: oneshot::Sender<Result<Delivered, SessionError>>,
}

/// `session.queue`.
#[derive(Debug)]
pub(crate) struct Change {
    pub(crate) generation: vessel_proto::restart::RestartGeneration,
    pub(crate) action: QueueChange,
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
    /// Captured at actual control admission, never reread after a stop.
    admission: vessel_proto::restart::RestartGeneration,
    /// Automatic queue progress inherits the last accepted input.
    accepted: vessel_proto::restart::RestartGeneration,
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
    /// Who waits to learn which turn their prompt started (`tickets`).
    pub(crate) tickets: tickets::Tickets,
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
            AgentEvent::TurnStarted { .. } => {
                self.turn = Some(seq);
                self.tickets.turn_started(seq);
            }
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

    /// What the agent said it takes (all false before it said).
    pub(crate) const fn abilities(&self) -> AgentAbilities {
        self.abilities
    }

    /// Both normal and context-bearing prompts must advance this counter
    /// only after the driver accepted their complete request.
    pub(crate) fn started_one(&mut self) {
        self.started = self.started.saturating_add(1);
    }

    /// Turns the actor started.
    pub(crate) const fn started(&self) -> u64 {
        self.started
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
            generation,
            content,
            delivery,
            by,
            turn,
            reply,
        } = order;
        self.desk.admission = generation;
        let result = self.send_as(content, delivery, by).await;
        if let (Ok(delivered), Some(ticket)) = (&result, turn) {
            let started = self.desk.started();
            self.desk.tickets.give(ticket, *delivered, started);
        }
        let _gone = reply.send(result);
    }

    /// A person's cancel: the queue is held (nothing waiting is sent after
    /// a stop), and the turn named, if it still runs, is cancelled.
    pub(crate) async fn cancel(&mut self, order: Cancel) {
        let Cancel { turn, reply } = order;
        if self.desk.queue.hold(QueueHold::Cancelled) {
            self.publish_queue().await;
        }
        let result = if self.parking.parked.is_some() {
            // Nothing runs on a parked agent (protocol 12): the hold is
            // the whole of the cancel, and nothing wakes it.
            Ok(())
        } else if turn.is_none() || turn == self.desk.turn {
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

    async fn send_as(
        &mut self,
        content: Vec<ContentBlock>,
        delivery: Delivery,
        by: Creator,
    ) -> Result<Delivered, SessionError> {
        match delivery {
            Delivery::Start => self.start(content).await.map(|()| Delivered::Started),
            // A prompt the node delivers on its own (pull request feedback,
            // a schedule, a starter's notice) does not try again a wake that
            // failed (protocol 12): it waits, the queue held, for a person.
            Delivery::Queue if self.wake_failed() => {
                self.check_attachments(&content)?;
                let entry = self.desk.queue.push(content, by)?;
                let _held = self.desk.queue.hold(QueueHold::Failed);
                self.publish_queue().await;
                Ok(Delivered::Queued { entry })
            }
            Delivery::Queue => {
                let idle = !self.desk.running() && self.desk.queue.is_empty();
                if idle && !self.desk.queue.is_held() {
                    return self.start(content).await.map(|()| Delivered::Started);
                }
                self.check_attachments(&content)?;
                let entry = self.desk.queue.push(content, by)?;
                self.publish_queue().await;
                Ok(Delivered::Queued { entry })
            }
            Delivery::Steer => self.steer(content, by).await,
            Delivery::Interrupt => self.interrupt(content).await,
        }
    }

    /// Start a turn with `content`, its attachments loaded first.
    async fn start(&mut self, content: Vec<ContentBlock>) -> Result<(), SessionError> {
        let attachments = self.load_attachments(&content).await?;
        self.start_loaded(content, attachments).await
    }

    async fn start_loaded(
        &mut self,
        content: Vec<ContentBlock>,
        attachments: Attachments,
    ) -> Result<(), SessionError> {
        if self.life.out.input_closed() {
            return Err(SessionError::ShuttingDown);
        }
        // Every prompt reaches the agent here: a parked one is started
        // again first (protocol 12).
        self.wake().await?;
        if self.life.out.input_closed() {
            return Err(SessionError::ShuttingDown);
        }
        let input = self.input.clone();
        self.pump(async move { input.prompt_with(content, attachments).await })
            .await
            .map_err(|_stopped| SessionError::Ended)??;
        self.desk.started_one();
        self.arm_restart().await?;
        Ok(())
    }

    /// A driver accepts a prompt only after emitting its start. Pump that
    /// already-emitted record before committing the generation, including
    /// when a very short turn ended before the driver's reply arrived.
    async fn arm_restart(&mut self) -> Result<(), SessionError> {
        if !self.life.out.restart_enabled() {
            return Ok(());
        }
        let (ticket, turn) = oneshot::channel();
        self.desk
            .tickets
            .give(ticket, Delivered::Started, self.desk.started());
        let turn = self
            .pump(turn)
            .await
            .map_err(|_stopped| SessionError::Ended)?
            .map_err(|_ended| SessionError::Ended)?;
        let change = vessel_proto::restart::RestartCheckpoint::Arm {
            generation: self.desk.admission,
            turn,
        };
        self.life.out.checkpoint_restart(change).await?;
        self.desk.accepted = self.desk.admission;
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
        let attachments = self.load_attachments(&content).await?;
        let input = self.input.clone();
        let sent = content.clone();
        let outcome = self
            .pump(async move { input.steer_with(sent, attachments).await })
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
    /// `content`. The queue is not held by this cancel. Its attachments are
    /// loaded first, so a prompt refused for them cancels nothing.
    async fn interrupt(&mut self, content: Vec<ContentBlock>) -> Result<Delivered, SessionError> {
        let attachments = self.load_attachments(&content).await?;
        if self.desk.running() {
            self.desk.interrupting = true;
            let over = self.cancel_and_wait().await;
            self.desk.interrupting = false;
            if !over? {
                return Err(DeliveryError::TurnDidNotEnd.into());
            }
        }
        self.start_loaded(content, attachments)
            .await
            .map(|()| Delivered::Started)
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
}
