//! Which turn a prompt started, for whoever asked to know (protocol 11,
//! ADR 0033: the starter told).
//!
//! [Decision Log]
//! - Purpose: the node tells an agent when the turn its prompt started
//!   finishes or waits for someone, about that turn exactly. Only the
//!   actor knows which turn that is: a prompt that started one, the
//!   running one it joined, or the turn its queue entry starts later
//!   (perhaps after a move, an edit, or a person's `send_now`).
//! - Constraint: `turn_started` is published from the agent's outputs,
//!   which the actor may read before or after the prompt's own answer
//!   (`pump`); a successful prompt has always emitted it before it
//!   answered (`vessel-agent`'s driver), so it is there or on its way.
//! - Alternatives: the node reading the log for the next `turn_started`
//!   after the prompt (wrong for a steer, which joins a turn already
//!   running, and for a queued entry that a later prompt overtakes with
//!   `send_now`); matching a turn by its content (two prompts of one text).
//! - Choice: a one-shot sender travels with the prompt. It is given the
//!   seq of the turn's `turn_started` as soon as the actor has it; a queued
//!   entry's waits under the entry's id until the entry starts a turn,
//!   and is dropped when the entry leaves the queue another way, or with
//!   the actor when the session ends, so the receiver learns "never".
//! - Cost: one map entry per queued prompt that asked (the node bounds how
//!   many), none for every other prompt.

use std::collections::HashMap;

use tokio::sync::oneshot;
use vessel_proto::{Delivered, EventSeq, PromptQueue, QueueEntryId};

/// Told the seq of the `turn_started` record of the turn a prompt started
/// or joined; dropped if it never starts one.
#[derive(Debug)]
pub(crate) enum TurnTicket {
    Legacy(oneshot::Sender<EventSeq>),
    Receipt(oneshot::Sender<super::submission::SubmissionTarget>),
}

impl TurnTicket {
    fn started(self, seq: EventSeq) {
        match self {
            Self::Legacy(ticket) => {
                let _gone = ticket.send(seq);
            }
            Self::Receipt(ticket) => {
                let _gone = ticket.send(super::submission::SubmissionTarget::Turn { started: seq });
            }
        }
    }

    fn withdrawn(self, entry: QueueEntryId, through: EventSeq) {
        match self {
            // Legacy callers retain their original closed-channel semantics.
            Self::Legacy(_ticket) => {}
            Self::Receipt(ticket) => {
                let _gone =
                    ticket.send(super::submission::SubmissionTarget::Withdrawn { entry, through });
            }
        }
    }
}

/// The tickets the actor holds.
#[derive(Debug, Default)]
pub(crate) struct Tickets {
    /// `turn_started` records observed, and the latest one's seq.
    seen: u64,
    last: Option<EventSeq>,
    /// For a turn started or joined whose `turn_started` is on its way:
    /// the turn's number among those the actor started (the record of the
    /// `n`th turn is the `n`th observed), and its ticket.
    awaiting: Vec<(u64, TurnTicket)>,
    /// For prompts waiting in the queue, by entry.
    queued: HashMap<QueueEntryId, TurnTicket>,
}

impl Tickets {
    /// A `turn_started` about to be published as `seq`: the tickets
    /// waiting for this turn's record are told.
    pub(crate) fn turn_started(&mut self, seq: EventSeq) {
        self.seen = self.seen.saturating_add(1);
        self.last = Some(seq);
        let seen = self.seen;
        let (told, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut self.awaiting)
            .into_iter()
            .partition(|(turn, _)| *turn <= seen);
        self.awaiting = waiting;
        for (_, ticket) in told {
            ticket.started(seq);
        }
    }

    /// `ticket`'s prompt was delivered as `delivered`; `started` is the
    /// number of turns the actor has started, its own included.
    ///
    /// A prompt that started a turn, or steered into the running one, is
    /// about the `started`th turn, whose `turn_started` is the `started`th
    /// observed. Counting, not "the next record", because an interrupt
    /// cancels a turn whose own record may still be on its way: the next
    /// record would be that turn's, not this prompt's (the code review of
    /// the join, finding 8).
    pub(crate) fn give(&mut self, ticket: TurnTicket, delivered: Delivered, started: u64) {
        match delivered {
            Delivered::Started | Delivered::Steered if started <= self.seen => self.tell(ticket),
            Delivered::Started | Delivered::Steered => self.awaiting.push((started, ticket)),
            Delivered::Queued { entry } => {
                let _replaced = self.queued.insert(entry, ticket);
            }
        }
    }

    /// The ticket of a queue entry about to be sent.
    pub(crate) fn take(&mut self, entry: QueueEntryId) -> Option<TurnTicket> {
        self.queued.remove(&entry)
    }

    /// A ticket back with its entry, which went back to the queue unsent.
    pub(crate) fn keep(&mut self, entry: QueueEntryId, ticket: TurnTicket) {
        let _replaced = self.queued.insert(entry, ticket);
    }

    /// Drop the tickets of entries no longer in `queue` (removed or
    /// cleared): those prompts start no turn.
    pub(crate) fn prune(&mut self, queue: &PromptQueue, through: EventSeq) {
        if self.queued.is_empty() {
            return;
        }
        for (entry, ticket) in std::mem::take(&mut self.queued) {
            if queue.entries.iter().any(|queued| queued.id == entry) {
                let _replaced = self.queued.insert(entry, ticket);
            } else {
                ticket.withdrawn(entry, through);
            }
        }
    }

    /// The latest record is this prompt's turn's (the actor has seen as
    /// many begin as it started).
    fn tell(&self, ticket: TurnTicket) {
        if let Some(seq) = self.last {
            ticket.started(seq);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket() -> (TurnTicket, oneshot::Receiver<EventSeq>) {
        let (sender, receiver) = oneshot::channel();
        (TurnTicket::Legacy(sender), receiver)
    }

    #[test]
    fn receipt_withdrawal_is_explicit_and_owner_loss_stays_unknown() {
        let mut tickets = Tickets::default();
        let entry = QueueEntryId::random();
        let (sender, mut removed) = oneshot::channel();
        tickets.give(TurnTicket::Receipt(sender), Delivered::Queued { entry }, 0);
        tickets.prune(&PromptQueue::default(), EventSeq::new(27));
        assert_eq!(
            removed.try_recv(),
            Ok(super::super::submission::SubmissionTarget::Withdrawn {
                entry,
                through: EventSeq::new(27)
            })
        );
        let (sender, mut lost) = oneshot::channel();
        tickets.give(TurnTicket::Receipt(sender), Delivered::Queued { entry }, 0);
        drop(tickets);
        assert_eq!(lost.try_recv(), Err(oneshot::error::TryRecvError::Closed));
    }

    #[test]
    fn receipt_turn_ticket_preserves_fast_completion_and_replacement_scope() {
        let mut tickets = Tickets::default();
        let (sender, mut target) = oneshot::channel();
        tickets.give(TurnTicket::Receipt(sender), Delivered::Started, 2);
        tickets.turn_started(EventSeq::new(3));
        assert!(target.try_recv().is_err());
        tickets.turn_started(EventSeq::new(8));
        assert_eq!(
            target.try_recv(),
            Ok(super::super::submission::SubmissionTarget::Turn {
                started: EventSeq::new(8)
            })
        );
        let (sender, mut fast) = oneshot::channel();
        tickets.give(TurnTicket::Receipt(sender), Delivered::Steered, 2);
        assert_eq!(
            fast.try_recv(),
            Ok(super::super::submission::SubmissionTarget::Turn {
                started: EventSeq::new(8)
            })
        );
    }

    #[test]
    fn a_started_prompt_learns_its_turn_whichever_comes_first() {
        let mut tickets = Tickets::default();
        // The record came while the prompt was sent.
        tickets.turn_started(EventSeq::new(4));
        let (sender, mut early) = ticket();
        tickets.give(sender, Delivered::Started, 1);
        assert_eq!(early.try_recv(), Ok(EventSeq::new(4)));
        // The record is still on its way.
        let (sender, mut late) = ticket();
        tickets.give(sender, Delivered::Started, 2);
        assert!(late.try_recv().is_err());
        tickets.turn_started(EventSeq::new(9));
        assert_eq!(late.try_recv(), Ok(EventSeq::new(9)));
    }

    #[test]
    fn a_steer_joins_the_running_turn() {
        let mut tickets = Tickets::default();
        tickets.turn_started(EventSeq::new(4));
        let (sender, mut joined) = ticket();
        tickets.give(sender, Delivered::Steered, 1);
        assert_eq!(joined.try_recv(), Ok(EventSeq::new(4)));
        // A turn started whose record is on its way: that one.
        let (sender, mut next) = ticket();
        tickets.give(sender, Delivered::Steered, 2);
        assert!(next.try_recv().is_err());
        tickets.turn_started(EventSeq::new(12));
        assert_eq!(next.try_recv(), Ok(EventSeq::new(12)));
    }

    #[test]
    fn a_queued_prompt_waits_and_a_removed_one_hears_never() {
        let mut tickets = Tickets::default();
        let kept = QueueEntryId::random();
        let removed = QueueEntryId::random();
        let (sender, mut waits) = ticket();
        tickets.give(sender, Delivered::Queued { entry: kept }, 0);
        let (sender, mut never) = ticket();
        tickets.give(sender, Delivered::Queued { entry: removed }, 0);
        let queue = PromptQueue {
            entries: vec![vessel_proto::QueuedPrompt {
                id: kept,
                content: Vec::new(),
                queued_at: chrono::DateTime::UNIX_EPOCH,
                revision: 0,
                by: vessel_proto::Creator::Owner,

                client_message_id: None,
            }],
            held: None,
        };
        tickets.prune(&queue, EventSeq::new(19));
        assert_eq!(never.try_recv(), Err(oneshot::error::TryRecvError::Closed));
        let sender = tickets.take(kept).unwrap();
        tickets.give(sender, Delivered::Started, 1);
        tickets.turn_started(EventSeq::new(20));
        assert_eq!(waits.try_recv(), Ok(EventSeq::new(20)));
    }

    /// An interrupt: the running turn (the first) was started, but its
    /// record was not yet seen when the prompt that interrupts it started
    /// the second. The first record to come is the cancelled turn's, and
    /// the ticket waits for the second's.
    #[test]
    fn an_interrupting_prompt_learns_its_own_turn_not_the_one_it_cancelled() {
        let mut tickets = Tickets::default();
        let (sender, mut own) = ticket();
        tickets.give(sender, Delivered::Started, 2);
        tickets.turn_started(EventSeq::new(3));
        assert!(own.try_recv().is_err(), "the cancelled turn's record");
        tickets.turn_started(EventSeq::new(8));
        assert_eq!(own.try_recv(), Ok(EventSeq::new(8)));
        // Both seen before the ticket is given: the latest is its turn.
        let mut seen = Tickets::default();
        seen.turn_started(EventSeq::new(3));
        seen.turn_started(EventSeq::new(8));
        let (sender, mut late) = ticket();
        seen.give(sender, Delivered::Started, 2);
        assert_eq!(late.try_recv(), Ok(EventSeq::new(8)));
    }
}
