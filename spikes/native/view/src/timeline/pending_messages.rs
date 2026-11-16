//! Optimistic messages reconcile by caller identity, never equal text.
//! This is session-owned presentation state, not a second delivery manager.

use std::collections::VecDeque;
use std::time::Instant;

use vessel_proto::timeline::{ClientMessageId, SubmitPromptReply};

/// Admission happens before preparing bytes or sending any node request.
pub const MAX_PENDING_MESSAGES: usize = 32;

/// Refusal leaves the composer payload untouched and sends nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingFull;

/// One caller submission remains until an exact echo or authoritative disposition.
#[derive(Clone)]
pub struct PendingMessage {
    /// Minted once at shared session ingress and retained across preparation/retry.
    pub message: ClientMessageId,
    /// Original visible text; attachment-only submissions may have no text.
    pub text: String,
    /// The first admission owns arrival timing, even when a receipt is retried.
    pub admitted_at: Instant,
    /// Keep the canonical reply vocabulary rather than inventing accepted states.
    pub receipt: Option<SubmitPromptReply>,
}

impl std::fmt::Debug for PendingMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PendingMessage")
            .field("message", &self.message)
            .field("text_bytes", &self.text.len())
            .field("receipt", &self.receipt)
            .finish_non_exhaustive()
    }
}

impl PendingMessage {
    /// An uncertain result remains visible and is never automatically resent.
    #[must_use]
    pub fn reconciling(&self) -> bool {
        matches!(self.receipt, Some(SubmitPromptReply::OutcomeUnknown { .. }))
    }
}

/// Finite per-session admission; all bytes and retry decisions stay with sending.
#[derive(Debug, Default)]
pub struct PendingMessages(VecDeque<PendingMessage>);

impl PendingMessages {
    /// Keep admission order, including two equal texts with different identities.
    /// The caller first checks already observed canonical IDs, so a replay after
    /// reconciliation cannot create a new optimistic row.
    ///
    /// # Errors
    /// Refuses a fresh identity when the pending-row bound is full, before I/O.
    pub fn admit(
        &mut self,
        message: ClientMessageId,
        text: &str,
        at: Instant,
    ) -> Result<(), PendingFull> {
        if self.0.iter().any(|pending| pending.message == message) {
            return Ok(());
        }
        if self.0.len() >= MAX_PENDING_MESSAGES {
            return Err(PendingFull);
        }
        self.0.push_back(PendingMessage {
            message,
            text: text.to_owned(),
            admitted_at: at,
            receipt: None,
        });
        Ok(())
    }

    /// A late acceptance cannot resurrect a row already replaced by its echo.
    /// False means the reply did not belong to the request's captured identity.
    #[must_use]
    pub fn receipt(&mut self, expected: &ClientMessageId, reply: &SubmitPromptReply) -> bool {
        if reply_id(reply) != expected {
            return false;
        }
        if self.0.iter().any(|pending| {
            &pending.message == expected
                && matches!(pending.receipt, Some(SubmitPromptReply::Accepted { .. }))
        }) {
            return true;
        }
        match reply {
            // Queued acceptance also waits for canonical observation; an RPC
            // response may arrive before the authoritative queue push.
            SubmitPromptReply::Accepted { receipt } => self.remember(&receipt.message, reply),
            SubmitPromptReply::OutcomeUnknown { message } => self.remember(message, reply),
            SubmitPromptReply::KeyConflict { message }
            | SubmitPromptReply::Capacity { message, .. } => {
                self.remove(message);
            }
        }
        true
    }

    /// Local preparation failure or an exact canonical echo removes only this id.
    /// An echo without a caller id must never settle a coincidentally equal text.
    pub fn remove(&mut self, message: &ClientMessageId) {
        self.0.retain(|pending| &pending.message != message);
    }

    /// Rows borrow their payload until the drawing boundary needs owned text.
    #[must_use]
    pub fn rows(&self) -> &VecDeque<PendingMessage> {
        &self.0
    }

    fn remember(&mut self, message: &ClientMessageId, reply: &SubmitPromptReply) {
        if let Some(pending) = self
            .0
            .iter_mut()
            .find(|pending| &pending.message == message)
        {
            // A durable acceptance is stronger than a later uncertain observation.
            if !matches!(pending.receipt, Some(SubmitPromptReply::Accepted { .. })) {
                pending.receipt = Some(reply.clone());
            }
        }
    }
}

fn reply_id(reply: &SubmitPromptReply) -> &ClientMessageId {
    match reply {
        SubmitPromptReply::Accepted { receipt } => &receipt.message,
        SubmitPromptReply::KeyConflict { message }
        | SubmitPromptReply::OutcomeUnknown { message }
        | SubmitPromptReply::Capacity { message, .. } => message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::timeline::PromptReceipt;
    use vessel_proto::{Delivered, QueueEntryId};

    fn id(text: &str) -> ClientMessageId {
        text.parse().unwrap()
    }

    #[test]
    fn identical_words_are_distinct_and_a_late_result_cannot_resurrect_an_echoed_row() {
        let mut pending = PendingMessages::default();
        let first = id("first");
        let second = id("second");
        pending
            .admit(first.clone(), "Again", Instant::now())
            .unwrap();
        pending
            .admit(second.clone(), "Again", Instant::now())
            .unwrap();
        pending.remove(&first);
        assert!(pending.receipt(
            &first,
            &SubmitPromptReply::Accepted {
                receipt: PromptReceipt {
                    message: first.clone(),
                    delivered: Delivered::Started,
                },
            }
        ));
        assert_eq!(pending.rows().len(), 1);
        assert_eq!(pending.rows().front().unwrap().message, second);
    }

    #[test]
    fn uncertainty_retains_the_original_row_and_acceptance_waits_for_the_exact_echo() {
        let mut pending = PendingMessages::default();
        let message = id("first");
        pending.admit(message.clone(), "", Instant::now()).unwrap();
        assert!(pending.receipt(
            &message,
            &SubmitPromptReply::OutcomeUnknown {
                message: message.clone(),
            }
        ));
        assert!(pending.rows().front().unwrap().reconciling());
        let accepted = SubmitPromptReply::Accepted {
            receipt: PromptReceipt {
                message: message.clone(),
                delivered: Delivered::Steered,
            },
        };
        assert!(pending.receipt(&message, &accepted));
        assert!(pending.receipt(
            &message,
            &SubmitPromptReply::OutcomeUnknown {
                message: message.clone(),
            }
        ));
        assert_eq!(pending.rows().len(), 1);
        assert!(!pending.rows().front().unwrap().reconciling());
        pending.remove(&message);
        assert!(pending.rows().is_empty());
    }

    #[test]
    fn queued_acceptance_hands_off_to_the_canonical_queue_and_local_failure_is_id_scoped() {
        let mut pending = PendingMessages::default();
        pending.admit(id("queued"), "One", Instant::now()).unwrap();
        pending.admit(id("failed"), "Two", Instant::now()).unwrap();
        assert!(pending.receipt(
            &id("queued"),
            &SubmitPromptReply::Accepted {
                receipt: PromptReceipt {
                    message: id("queued"),
                    delivered: Delivered::Queued {
                        entry: QueueEntryId::random(),
                    },
                },
            }
        ));
        assert_eq!(pending.rows().len(), 2);
        pending.remove(&id("queued")); // Exact client ID in authoritative queue.
        assert_eq!(pending.rows().len(), 1);
        pending.remove(&id("failed"));
        assert!(pending.rows().is_empty());
    }

    #[test]
    fn full_admission_refuses_before_io_but_a_retry_never_consumes_another_slot() {
        let mut pending = PendingMessages::default();
        for index in 0..MAX_PENDING_MESSAGES {
            pending
                .admit(id(&format!("message-{index}")), "text", Instant::now())
                .unwrap();
        }
        assert_eq!(
            pending.admit(id("overflow"), "text", Instant::now()),
            Err(PendingFull)
        );
        assert_eq!(
            pending.admit(id("message-0"), "text", Instant::now()),
            Ok(())
        );
        assert_eq!(pending.rows().len(), MAX_PENDING_MESSAGES);
    }

    #[test]
    fn a_reply_bound_to_another_request_cannot_remove_either_pending_message() {
        let mut pending = PendingMessages::default();
        pending.admit(id("first"), "Same", Instant::now()).unwrap();
        pending.admit(id("second"), "Same", Instant::now()).unwrap();
        assert!(!pending.receipt(
            &id("first"),
            &SubmitPromptReply::KeyConflict {
                message: id("second"),
            }
        ));
        assert_eq!(pending.rows().len(), 2);
        assert!(pending.rows().iter().all(|row| row.receipt.is_none()));
    }

    #[test]
    fn a_durable_acceptance_survives_a_late_conflict_for_the_same_identity() {
        let mut pending = PendingMessages::default();
        let message = id("first");
        pending
            .admit(message.clone(), "Original", Instant::now())
            .unwrap();
        let accepted = SubmitPromptReply::Accepted {
            receipt: PromptReceipt {
                message: message.clone(),
                delivered: Delivered::Started,
            },
        };
        assert!(pending.receipt(&message, &accepted));
        assert!(pending.receipt(
            &message,
            &SubmitPromptReply::KeyConflict {
                message: message.clone()
            }
        ));
        assert_eq!(pending.rows().len(), 1);
        assert_eq!(pending.rows().front().unwrap().receipt, Some(accepted));
    }
}
