//! The existing ticket owner retains original turn and exact accepted echo.
//! One preparing slot is registered before dispatch. Canonical receipt tickets
//! stay bounded by the existing queue capacity, including caller loss; they never
//! resolve from foreign events, later-turn guesses or a missing acknowledgement.

mod receipt;
use receipt::ReceiptTicket;
use std::collections::HashMap;
use tokio::sync::oneshot;
use vessel_proto::timeline::ClientMessageId;
use vessel_proto::{Delivered, EventSeq, PromptQueue, QueueEntryId};

#[derive(Debug)]
pub(crate) enum TurnTicket {
    Legacy(oneshot::Sender<EventSeq>),
    Receipt(ReceiptTicket),
}

impl TurnTicket {
    pub(crate) fn receipt(
        message: ClientMessageId,
        reply: oneshot::Sender<super::submission::SubmissionTarget>,
    ) -> Self {
        Self::Receipt(ReceiptTicket::new(message, reply))
    }

    pub(crate) const fn is_receipt(&self) -> bool {
        matches!(self, Self::Receipt(_))
    }

    fn started(self, seq: EventSeq) -> Option<Self> {
        match self {
            Self::Legacy(ticket) => {
                let _gone = ticket.send(seq);
                None
            }
            Self::Receipt(mut ticket) => {
                if ticket.started(seq) {
                    None
                } else {
                    Some(Self::Receipt(ticket))
                }
            }
        }
    }

    fn echoed(&mut self, message: &ClientMessageId, seq: EventSeq) -> bool {
        match self {
            Self::Legacy(_) => false,
            Self::Receipt(ticket) => ticket.echoed(message, seq),
        }
    }

    fn withdrawn(self, entry: QueueEntryId, through: EventSeq) {
        match self {
            Self::Legacy(_ticket) => {}
            Self::Receipt(ticket) => ticket.withdrawn(entry, through),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Tickets {
    seen: u64,
    last: Option<EventSeq>,
    // None is an admitted uncertain dispatch whose target cannot be guessed.
    awaiting: Vec<(Option<u64>, TurnTicket)>,
    queued: HashMap<QueueEntryId, TurnTicket>,
    preparing: Option<TurnTicket>,
}

impl Tickets {
    /// This same owner must take the original ticket BEFORE any dispatch await.
    /// Refusal retains it and proves no provider/queue/cancel work was admitted.
    pub(crate) fn prepare(&mut self, ticket: TurnTicket) -> Result<(), TurnTicket> {
        let receipts = self
            .awaiting
            .iter()
            .filter(|(_, ticket)| ticket.is_receipt())
            .count()
            .saturating_add(
                self.queued
                    .values()
                    .filter(|ticket| ticket.is_receipt())
                    .count(),
            );
        if self.preparing.is_some() || (ticket.is_receipt() && receipts >= PromptQueue::MAX_ENTRIES)
        {
            return Err(ticket);
        }
        self.preparing = Some(ticket);
        Ok(())
    }

    pub(crate) fn take_prepared(&mut self) -> Option<TurnTicket> {
        self.preparing.take()
    }

    pub(crate) fn finish_prepared(&mut self, delivered: Delivered, started: u64) {
        if let Some(ticket) = self.preparing.take() {
            self.give(ticket, delivered, started);
        }
    }

    /// Possibly sent work retains its bounded ticket until owner settlement/end.
    /// No target is inferred even if a later unrelated turn is published.
    pub(crate) fn uncertain(&mut self) {
        if let Some(ticket) = self.preparing.take() {
            self.keep_uncertain(ticket);
        }
    }

    pub(crate) fn keep_uncertain(&mut self, ticket: TurnTicket) {
        if ticket.is_receipt() {
            self.awaiting.push((None, ticket));
        }
    }

    /// Called ONLY after publication of a closed trusted accepted-input echo.
    pub(crate) fn echoed(&mut self, message: &ClientMessageId, seq: EventSeq) {
        if let Some(ticket) = &mut self.preparing {
            let _settled = ticket.echoed(message, seq);
        }
        self.awaiting
            .retain_mut(|(_, ticket)| !ticket.echoed(message, seq));
    }

    pub(crate) fn turn_started(&mut self, seq: EventSeq) {
        self.seen = self.seen.saturating_add(1);
        self.last = Some(seq);
        let seen = self.seen;
        let (told, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut self.awaiting)
            .into_iter()
            .partition(|(turn, _)| turn.is_some_and(|turn| turn <= seen));
        self.awaiting = waiting;
        for (turn, ticket) in told {
            if let Some(ticket) = ticket.started(seq) {
                self.awaiting.push((turn, ticket));
            }
        }
    }

    /// Counting retains the original turn even when an interrupted turn's own
    /// start is still buffered. Echo sequence is a distinct exact record.
    pub(crate) fn give(&mut self, ticket: TurnTicket, delivered: Delivered, started: u64) {
        match delivered {
            Delivered::Started | Delivered::Steered if started <= self.seen => {
                if let Some(seq) = self.last {
                    if let Some(ticket) = ticket.started(seq) {
                        self.awaiting.push((Some(started), ticket));
                    }
                } else {
                    self.awaiting.push((None, ticket));
                }
            }
            Delivered::Started | Delivered::Steered => self.awaiting.push((Some(started), ticket)),
            Delivered::Queued { entry } => {
                let _replaced = self.queued.insert(entry, ticket);
            }
        }
    }

    pub(crate) fn take(&mut self, entry: QueueEntryId) -> Option<TurnTicket> {
        self.queued.remove(&entry)
    }
    pub(crate) fn keep(&mut self, entry: QueueEntryId, ticket: TurnTicket) {
        let _replaced = self.queued.insert(entry, ticket);
    }

    /// Only an entry still in this unsent queue owner can become Withdrawn.
    /// A preparing/uncertain dispatch has been moved out before this walk.
    pub(crate) fn prune(&mut self, queue: &PromptQueue, through: EventSeq) {
        for (entry, ticket) in std::mem::take(&mut self.queued) {
            if queue.entries.iter().any(|queued| queued.id == entry) {
                let _replaced = self.queued.insert(entry, ticket);
            } else {
                ticket.withdrawn(entry, through);
            }
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
    fn preparing_and_uncertain_tickets_do_not_become_false_withdrawals() {
        let mut tickets = Tickets::default();
        let message = ClientMessageId::random();
        let (sent, mut result) = oneshot::channel();
        tickets
            .prepare(TurnTicket::receipt(message.clone(), sent))
            .unwrap();
        tickets.prune(&PromptQueue::default(), EventSeq::new(19));
        assert_eq!(result.try_recv(), Err(oneshot::error::TryRecvError::Empty));
        tickets.uncertain();
        tickets.turn_started(EventSeq::new(31));
        tickets.echoed(&message, EventSeq::new(63));
        tickets.prune(&PromptQueue::default(), EventSeq::new(99));
        assert_eq!(result.try_recv(), Err(oneshot::error::TryRecvError::Empty));
        drop(tickets);
        assert_eq!(result.try_recv(), Err(oneshot::error::TryRecvError::Closed));
    }

    #[test]
    fn queued_tickets_move_to_the_original_preparing_slot_before_echo() {
        let mut tickets = Tickets::default();
        let message = ClientMessageId::random();
        let entry = QueueEntryId::random();
        let (sent, mut result) = oneshot::channel();
        tickets
            .prepare(TurnTicket::receipt(message.clone(), sent))
            .unwrap();
        tickets.finish_prepared(Delivered::Queued { entry }, 0);
        let original = tickets.take(entry).unwrap();
        tickets.prepare(original).unwrap();
        tickets.turn_started(EventSeq::new(17));
        tickets.echoed(&message, EventSeq::new(17));
        tickets.finish_prepared(Delivered::Started, 1);
        assert_eq!(
            result.try_recv(),
            Ok(super::super::submission::SubmissionTarget::Turn {
                started: EventSeq::new(17),
                echo: EventSeq::new(17)
            })
        );
        assert!(tickets.awaiting.is_empty());
        assert!(tickets.queued.is_empty());
    }

    #[test]
    fn full_receipt_capacity_keeps_original_tickets_and_allows_existing_transfer() {
        let mut tickets = Tickets::default();
        let mut receivers = Vec::new();
        let first = QueueEntryId::random();
        for index in 0..PromptQueue::MAX_ENTRIES {
            let (sent, received) = oneshot::channel();
            tickets
                .prepare(TurnTicket::receipt(ClientMessageId::random(), sent))
                .unwrap();
            let entry = if index == 0 {
                first
            } else {
                QueueEntryId::random()
            };
            tickets.finish_prepared(Delivered::Queued { entry }, 0);
            receivers.push(received);
        }
        let (sent, mut refused) = oneshot::channel();
        let original = tickets
            .prepare(TurnTicket::receipt(ClientMessageId::random(), sent))
            .unwrap_err();
        assert_eq!(refused.try_recv(), Err(oneshot::error::TryRecvError::Empty));
        let queued = tickets.take(first).unwrap();
        tickets.prepare(queued).unwrap();
        tickets.finish_prepared(Delivered::Queued { entry: first }, 0);
        assert_eq!(tickets.queued.len(), PromptQueue::MAX_ENTRIES);
        assert!(
            receivers
                .iter_mut()
                .all(|received| received.try_recv() == Err(oneshot::error::TryRecvError::Empty))
        );
        drop(original);
        assert_eq!(
            refused.try_recv(),
            Err(oneshot::error::TryRecvError::Closed)
        );
    }

    #[test]
    fn losing_an_observer_does_not_release_a_ticket_before_actual_echo_settlement() {
        let mut tickets = Tickets::default();
        let message = ClientMessageId::random();
        let (sent, received) = oneshot::channel();
        tickets
            .prepare(TurnTicket::receipt(message.clone(), sent))
            .unwrap();
        tickets.turn_started(EventSeq::new(17));
        tickets.finish_prepared(Delivered::Started, 1);
        drop(received);
        assert_eq!(tickets.awaiting.len(), 1);
        tickets.echoed(&message, EventSeq::new(17));
        assert!(tickets.awaiting.is_empty());
    }

    #[test]
    fn receipt_withdrawal_is_explicit_and_owner_loss_stays_unknown() {
        let mut tickets = Tickets::default();
        let entry = QueueEntryId::random();
        let (sender, mut removed) = oneshot::channel();
        tickets.give(
            TurnTicket::receipt(ClientMessageId::random(), sender),
            Delivered::Queued { entry },
            0,
        );
        tickets.prune(&PromptQueue::default(), EventSeq::new(27));
        assert_eq!(
            removed.try_recv(),
            Ok(super::super::submission::SubmissionTarget::Withdrawn {
                entry,
                through: EventSeq::new(27)
            })
        );
        let (sender, mut lost) = oneshot::channel();
        tickets.give(
            TurnTicket::receipt(ClientMessageId::random(), sender),
            Delivered::Queued { entry },
            0,
        );
        drop(tickets);
        assert_eq!(lost.try_recv(), Err(oneshot::error::TryRecvError::Closed));
    }

    #[test]
    fn receipt_turn_ticket_preserves_fast_completion_and_replacement_scope() {
        let mut tickets = Tickets::default();
        let (sender, mut target) = oneshot::channel();
        let message = ClientMessageId::random();
        tickets
            .prepare(TurnTicket::receipt(message.clone(), sender))
            .unwrap();
        tickets.echoed(&message, EventSeq::new(8));
        tickets.finish_prepared(Delivered::Started, 2);
        tickets.turn_started(EventSeq::new(3));
        assert!(target.try_recv().is_err());
        tickets.turn_started(EventSeq::new(8));
        assert_eq!(
            target.try_recv(),
            Ok(super::super::submission::SubmissionTarget::Turn {
                started: EventSeq::new(8),
                echo: EventSeq::new(8)
            })
        );
        let (sender, mut fast) = oneshot::channel();
        let message = ClientMessageId::random();
        tickets
            .prepare(TurnTicket::receipt(message.clone(), sender))
            .unwrap();
        tickets.finish_prepared(Delivered::Steered, 2);
        assert_eq!(fast.try_recv(), Err(oneshot::error::TryRecvError::Empty));
        tickets.echoed(&message, EventSeq::new(99));
        assert_eq!(
            fast.try_recv(),
            Ok(super::super::submission::SubmissionTarget::Turn {
                started: EventSeq::new(8),
                echo: EventSeq::new(99)
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
                provenance: None,
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
