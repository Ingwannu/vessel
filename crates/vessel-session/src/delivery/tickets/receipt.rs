//! One original receipt ticket joins its exact accepted echo with its real turn.
//! Neither order can replace that turn with a later one or invent a withdrawal.
use tokio::sync::oneshot;
use vessel_proto::timeline::ClientMessageId;
use vessel_proto::{EventSeq, QueueEntryId};

use super::super::submission::SubmissionTarget;

#[derive(Debug)]
pub(crate) struct ReceiptTicket {
    message: ClientMessageId,
    reply: Option<oneshot::Sender<SubmissionTarget>>,
    turn: Option<EventSeq>,
    echo: Option<EventSeq>,
}

impl ReceiptTicket {
    pub(super) fn new(message: ClientMessageId, reply: oneshot::Sender<SubmissionTarget>) -> Self {
        Self {
            message,
            reply: Some(reply),
            turn: None,
            echo: None,
        }
    }

    pub(super) fn started(&mut self, started: EventSeq) -> bool {
        // The ticket's original binding is immutable even if another start is
        // observed while this input's own echo is still waiting for publication.
        let _original = self.turn.get_or_insert(started);
        self.settle()
    }

    pub(super) fn echoed(&mut self, message: &ClientMessageId, echo: EventSeq) -> bool {
        if message != &self.message {
            return false;
        }
        let _original = self.echo.get_or_insert(echo);
        self.settle()
    }

    fn settle(&mut self) -> bool {
        let (Some(started), Some(echo)) = (self.turn, self.echo) else {
            return false;
        };
        if let Some(reply) = self.reply.take() {
            let _observer_gone = reply.send(SubmissionTarget::Turn { started, echo });
        }
        true
    }

    pub(super) fn withdrawn(mut self, entry: QueueEntryId, through: EventSeq) {
        if let Some(reply) = self.reply.take() {
            let _observer_gone = reply.send(SubmissionTarget::Withdrawn { entry, through });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot::error::TryRecvError;

    #[test]
    fn echo_first_never_settles_before_the_original_turn_binding() {
        let message = ClientMessageId::random();
        let (sent, mut result) = oneshot::channel();
        let mut ticket = ReceiptTicket::new(message.clone(), sent);
        assert!(!ticket.echoed(&message, EventSeq::new(99)));
        assert_eq!(result.try_recv(), Err(TryRecvError::Empty));
        assert!(ticket.started(EventSeq::new(17)));
        assert_eq!(
            result.try_recv(),
            Ok(SubmissionTarget::Turn {
                started: EventSeq::new(17),
                echo: EventSeq::new(99)
            })
        );
    }

    #[test]
    fn binding_first_keeps_that_turn_until_this_exact_input_echo_arrives() {
        let message = ClientMessageId::random();
        let (sent, mut result) = oneshot::channel();
        let mut ticket = ReceiptTicket::new(message.clone(), sent);
        assert!(!ticket.started(EventSeq::new(17)));
        assert!(!ticket.started(EventSeq::new(31)));
        assert!(!ticket.echoed(&ClientMessageId::random(), EventSeq::new(63)));
        assert_eq!(result.try_recv(), Err(TryRecvError::Empty));
        assert!(ticket.echoed(&message, EventSeq::new(99)));
        assert_eq!(
            result.try_recv(),
            Ok(SubmissionTarget::Turn {
                started: EventSeq::new(17),
                echo: EventSeq::new(99)
            })
        );
    }

    #[test]
    fn closed_or_missing_echo_never_manufactures_no_send_or_a_turn() {
        let (sent, mut result) = oneshot::channel();
        let mut ticket = ReceiptTicket::new(ClientMessageId::random(), sent);
        assert!(!ticket.started(EventSeq::new(17)));
        drop(ticket);
        assert_eq!(result.try_recv(), Err(TryRecvError::Closed));
    }
}
