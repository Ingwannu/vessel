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
pub(crate) type TurnTicket = oneshot::Sender<EventSeq>;

/// The tickets the actor holds.
#[derive(Debug, Default)]
pub(crate) struct Tickets {
    /// `turn_started` records observed, and the latest one's seq.
    seen: u64,
    last: Option<EventSeq>,
    /// For a turn started or joined whose `turn_started` is on its way.
    awaiting: Vec<TurnTicket>,
    /// For prompts waiting in the queue, by entry.
    queued: HashMap<QueueEntryId, TurnTicket>,
}

impl Tickets {
    /// How many `turn_started` records were observed; taken before a
    /// delivery, it tells [`give`](Self::give) whether the prompt's own
    /// came meanwhile.
    pub(crate) const fn seen(&self) -> u64 {
        self.seen
    }

    /// A `turn_started` about to be published as `seq`.
    pub(crate) fn turn_started(&mut self, seq: EventSeq) {
        self.seen = self.seen.saturating_add(1);
        self.last = Some(seq);
        for ticket in self.awaiting.drain(..) {
            let _gone = ticket.send(seq);
        }
    }

    /// `ticket`'s prompt was delivered as `delivered`. `seen_before` is
    /// [`seen`](Self::seen) before the delivery; `started` the turns the
    /// actor has started so far.
    pub(crate) fn give(
        &mut self,
        ticket: TurnTicket,
        delivered: Delivered,
        seen_before: u64,
        started: u64,
    ) {
        match delivered {
            // Its turn's record came while the prompt was sent, or is
            // the next to come.
            Delivered::Started if self.seen > seen_before => self.tell(ticket),
            // The running turn's record is on its way when the actor
            // started more turns than it has seen begin.
            Delivered::Steered if started <= self.seen => self.tell(ticket),
            Delivered::Started | Delivered::Steered => self.awaiting.push(ticket),
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
    pub(crate) fn prune(&mut self, queue: &PromptQueue) {
        if self.queued.is_empty() {
            return;
        }
        self.queued
            .retain(|entry, _| queue.entries.iter().any(|queued| queued.id == *entry));
    }

    fn tell(&mut self, ticket: TurnTicket) {
        match self.last {
            Some(seq) => {
                let _gone = ticket.send(seq);
            }
            None => self.awaiting.push(ticket),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket() -> (TurnTicket, oneshot::Receiver<EventSeq>) {
        oneshot::channel()
    }

    #[test]
    fn a_started_prompt_learns_its_turn_whichever_comes_first() {
        let mut tickets = Tickets::default();
        // The record came while the prompt was sent.
        let before = tickets.seen();
        tickets.turn_started(EventSeq::new(4));
        let (sender, mut early) = ticket();
        tickets.give(sender, Delivered::Started, before, 1);
        assert_eq!(early.try_recv(), Ok(EventSeq::new(4)));
        // The record is still on its way.
        let before = tickets.seen();
        let (sender, mut late) = ticket();
        tickets.give(sender, Delivered::Started, before, 2);
        assert!(late.try_recv().is_err());
        tickets.turn_started(EventSeq::new(9));
        assert_eq!(late.try_recv(), Ok(EventSeq::new(9)));
    }

    #[test]
    fn a_steer_joins_the_running_turn() {
        let mut tickets = Tickets::default();
        tickets.turn_started(EventSeq::new(4));
        let (sender, mut joined) = ticket();
        tickets.give(sender, Delivered::Steered, tickets.seen(), 1);
        assert_eq!(joined.try_recv(), Ok(EventSeq::new(4)));
        // A turn started whose record is on its way: that one.
        let (sender, mut next) = ticket();
        tickets.give(sender, Delivered::Steered, tickets.seen(), 2);
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
        tickets.give(sender, Delivered::Queued { entry: kept }, 0, 0);
        let (sender, mut never) = ticket();
        tickets.give(sender, Delivered::Queued { entry: removed }, 0, 0);
        let queue = PromptQueue {
            entries: vec![vessel_proto::QueuedPrompt {
                id: kept,
                content: Vec::new(),
                queued_at: chrono::DateTime::UNIX_EPOCH,
                revision: 0,
                by: vessel_proto::Creator::Owner,
            }],
            held: None,
        };
        tickets.prune(&queue);
        assert_eq!(never.try_recv(), Err(oneshot::error::TryRecvError::Closed));
        let sender = tickets.take(kept).unwrap();
        let before = tickets.seen();
        tickets.give(sender, Delivered::Started, before, 1);
        tickets.turn_started(EventSeq::new(20));
        assert_eq!(waits.try_recv(), Ok(EventSeq::new(20)));
    }
}
