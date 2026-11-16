//! Identified prompt value types; registration waits for the shared method table.
//! The existing unkeyed prompt call never silently substitutes for this contract.

use serde::{Deserialize, Serialize};

use super::ClientMessageId;
use super::admission::AdmissionCapacity;
use super::{SubmissionAcceptedKind, SubmissionDelivery, SubmissionRefusal};
use crate::{ContentBlock, SessionId};

/// A caller retains this identity across retries of the same validated request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SubmitPrompt {
    /// The authorized owning session, checked before receipt admission.
    pub session: SessionId,
    /// Caller-created identity; it is not a transport request id.
    pub message: ClientMessageId,
    /// Content is fingerprinted after ordinary input validation.
    pub content: Vec<ContentBlock>,
    /// Retries cannot change the requested delivery behavior under the same identity.
    pub delivery: SubmissionDelivery,
    /// A producer-advertised native intent, validated against the same original text.
    /// It uses this same identity/fingerprint and requires Start routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_command: Option<crate::native_commands::NativeCommandIntent>,
}

/// Accepted delivery preserves the exact queued entry or direct disposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PromptReceipt {
    /// The caller's identity, echoed for optimistic row reconciliation.
    pub message: ClientMessageId,
    /// Accepted means delivered or queued, not completed provider work.
    pub accepted: SubmissionAcceptedKind,
}

/// Replay protection distinguishes accepted delivery from conflict and uncertainty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SubmitPromptReply {
    /// A new delivery or replay of its exact stored acceptance.
    Accepted {
        /// The original accepted result, unchanged on replay.
        receipt: PromptReceipt,
    },
    /// Preflight refused before reserving an identity; no external work happened.
    NotAdmitted {
        /// Identity remains available for a later explicitly submitted request.
        message: ClientMessageId,
        /// Exact validation/current-run capability refusal.
        reason: SubmissionRefusal,
    },
    /// A stable no-effect refusal, replayed under the same protected identity.
    Refused {
        /// Caller identity whose exact request was refused.
        message: ClientMessageId,
        /// Closed reason; no provider stderr, raw RPC error or objective output.
        reason: SubmissionRefusal,
    },
    /// An identity cannot be reused for different content, delivery or creator.
    KeyConflict {
        /// Identity whose fingerprint differs.
        message: ClientMessageId,
    },
    /// Pending may already have reached the provider; automatic resend is forbidden.
    OutcomeUnknown {
        /// Identity whose uncertain outcome remains protected.
        message: ClientMessageId,
    },
    /// New admission was refused before any provider or queue work.
    Capacity {
        /// Identity was not admitted and has no retained receipt.
        message: ClientMessageId,
        /// Which finite resource refused it.
        capacity: AdmissionCapacity,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QueueEntryId;

    #[test]
    fn accepted_receipt_keeps_the_exact_queue_identity_on_roundtrip() {
        let entry = QueueEntryId::random();
        let reply = SubmitPromptReply::Accepted {
            receipt: PromptReceipt {
                message: "caller-message".parse().unwrap(),
                accepted: SubmissionAcceptedKind::Prompt {
                    disposition: super::super::PromptDisposition::Queued { entry },
                },
            },
        };
        let encoded = serde_json::to_string(&reply).unwrap();
        assert_eq!(
            serde_json::from_str::<SubmitPromptReply>(&encoded).unwrap(),
            reply
        );
        let unknown: SubmitPromptReply =
            serde_json::from_str(r#"{"status":"outcome_unknown","message":"caller-message"}"#)
                .unwrap();
        assert!(matches!(unknown, SubmitPromptReply::OutcomeUnknown { .. }));
    }
}
