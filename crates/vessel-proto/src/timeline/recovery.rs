//! Read-only recovery values; method registration and owner wiring are separate.
//! Missing and uncertain outcomes never authorize another delivery attempt.

use serde::{Deserialize, Serialize};

use super::{ClientMessageId, PromptReceipt, SubmissionRefusal, SubmitPrompt};
use crate::{EventSeq, QueueEntryId};

/// Observe the complete original request under the authenticated caller's Creator.
/// It contains no internal namespace or caller-supplied authority selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ObservePromptReceipt {
    /// Unmodified original content, delivery and intent used for the fingerprint.
    pub request: SubmitPrompt,
}

/// Exact actor-derived target metadata, separate from immutable acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PromptReceiptBinding {
    /// No exact target is proved; absence never proves no external effect.
    Unobserved,
    /// Exact accepted queue entry, which may already have advanced.
    Queued {
        /// Original queue identity.
        entry: QueueEntryId,
    },
    /// Actual owning actor ticket observed this canonical turn start.
    Turn {
        /// Original turn-start record, retained even after fast completion.
        started: EventSeq,
    },
    /// Owning actor explicitly removed this entry before send.
    Withdrawn {
        /// Exact entry whose no-send removal was committed.
        entry: QueueEntryId,
    },
    /// Native command acceptance binds an echo, never a model turn.
    Command {
        /// Actual persisted command echo.
        echo: EventSeq,
    },
}

/// Observation cannot reserve, enqueue, reload attachments or wake a stored session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ObservePromptReceiptReply {
    /// No retained key was observed; automatic resend/fresh-id fallback is forbidden.
    Unobserved {
        /// Original caller-retained identity.
        message: ClientMessageId,
    },
    /// Possibly sent work remains protected by the retained owner.
    OutcomeUnknown {
        /// Protected identity.
        message: ClientMessageId,
    },
    /// Creator or original fingerprint differs; no overwrite or effect is admitted.
    KeyConflict {
        /// Conflicting identity.
        message: ClientMessageId,
    },
    /// Replay of the same retained no-effect refusal.
    Refused {
        /// Protected identity.
        message: ClientMessageId,
        /// Original closed refusal, unchanged by later capabilities.
        reason: SubmissionRefusal,
    },
    /// Original immutable acceptance and separately retained target fact.
    Accepted {
        /// Same accepted receipt returned by initial submission.
        receipt: PromptReceipt,
        /// Only the owning live actor can advance this correlation.
        binding: PromptReceiptBinding,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::{PromptDisposition, SubmissionAcceptedKind, SubmissionDelivery};
    use crate::{ContentBlock, SessionId};

    #[test]
    fn observation_carries_original_input_without_an_authority_selector() {
        let original = SubmitPrompt {
            session: SessionId::random(),
            message: ClientMessageId::random(),
            content: vec![ContentBlock::Text {
                text: " original ".to_owned(),
            }],
            delivery: SubmissionDelivery::SteerOrInterrupt,
            native_command: None,
        };
        let query = ObservePromptReceipt {
            request: original.clone(),
        };
        let encoded = serde_json::to_string(&query).unwrap();
        assert_eq!(
            serde_json::from_str::<ObservePromptReceipt>(&encoded)
                .unwrap()
                .request,
            original
        );
        assert!(!encoded.contains("creator"));
        assert!(!encoded.contains("namespace"));
    }

    #[test]
    fn observation_preserves_exact_binding_and_distinguishes_missing_from_uncertain() {
        let message = ClientMessageId::random();
        let entry = QueueEntryId::random();
        let receipt = PromptReceipt {
            message: message.clone(),
            accepted: SubmissionAcceptedKind::Prompt {
                disposition: PromptDisposition::Queued { entry },
            },
        };
        for binding in [
            PromptReceiptBinding::Unobserved,
            PromptReceiptBinding::Queued { entry },
            PromptReceiptBinding::Turn {
                started: EventSeq::new(91),
            },
            PromptReceiptBinding::Withdrawn { entry },
        ] {
            let reply = ObservePromptReceiptReply::Accepted {
                receipt: receipt.clone(),
                binding,
            };
            let encoded = serde_json::to_string(&reply).unwrap();
            assert_eq!(
                serde_json::from_str::<ObservePromptReceiptReply>(&encoded).unwrap(),
                reply
            );
        }
        let missing = serde_json::to_string(&ObservePromptReceiptReply::Unobserved {
            message: message.clone(),
        })
        .unwrap();
        let uncertain =
            serde_json::to_string(&ObservePromptReceiptReply::OutcomeUnknown { message }).unwrap();
        assert_ne!(missing, uncertain);
        assert!(matches!(
            serde_json::from_str::<ObservePromptReceiptReply>(&missing).unwrap(),
            ObservePromptReceiptReply::Unobserved { .. }
        ));
        assert!(matches!(
            serde_json::from_str::<ObservePromptReceiptReply>(&uncertain).unwrap(),
            ObservePromptReceiptReply::OutcomeUnknown { .. }
        ));
    }

    #[test]
    fn native_command_observation_keeps_the_actual_echo_without_a_turn_binding() {
        let echo = EventSeq::new(92);
        let reply = ObservePromptReceiptReply::Accepted {
            receipt: PromptReceipt {
                message: ClientMessageId::random(),
                accepted: SubmissionAcceptedKind::Command {
                    status: crate::native_commands::NativeCommandStatus::CompactStarted,
                    echo,
                },
            },
            binding: PromptReceiptBinding::Command { echo },
        };
        let encoded = serde_json::to_string(&reply).unwrap();
        assert_eq!(
            serde_json::from_str::<ObservePromptReceiptReply>(&encoded).unwrap(),
            reply
        );
        assert!(!encoded.contains("\"started\":"));
    }
}
