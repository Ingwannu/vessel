//! Exact-turn receipts for origin-side cross-node finish notifications.
use crate::{ContentBlock, Delivered, Delivery, EventSeq, SessionId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Opaque node-minted receipt, bound to its authenticated opener and session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct PromptReceiptId(Uuid);
impl PromptReceiptId {
    /// Mint a fresh identifier; possession alone grants no authority.
    #[must_use]
    pub fn random() -> Self {
        Self(Uuid::new_v4())
    }
}
/// Deliver once and retain the exact turn started or joined by this prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrackPrompt {
    /// The structured session to operate.
    pub session: SessionId,
    /// Prompt content, validated by the existing session/attachment path.
    pub content: Vec<ContentBlock>,
    /// Queue, steer, interrupt or start using the ordinary delivery rules.
    pub delivery: Delivery,
}
/// A receipt is returned immediately even when the prompt waits in a queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrackedPrompt {
    /// What happened to the prompt.
    pub delivered: Delivered,
    /// Read as the same opener on the same session, then explicitly close.
    pub receipt: PromptReceiptId,
}
/// Current exact-turn receipt state; no provider fields cross this boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PromptReceiptState {
    /// Still queued, with no turn number to follow yet.
    Pending,
    /// The exact turn-start record, including a steer into an existing turn.
    Started {
        /// Read this session's durable log from this record.
        turn: EventSeq,
    },
    /// The queue entry was removed or the session ended before it ran.
    Removed,
}
/// Read one opener-bound receipt; the server may long-poll a pending state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadPromptReceipt {
    /// Must match the receipt's operated session.
    pub session: SessionId,
    /// The node-issued receipt.
    pub receipt: PromptReceiptId,
}
/// Release a receipt after obtaining its exact turn; otherwise it expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClosePromptReceipt {
    /// Must match the receipt's operated session.
    pub session: SessionId,
    /// The node-issued receipt.
    pub receipt: PromptReceiptId,
}

/// Negotiate the complete route before a notifying create can start a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CheckPromptTracking {}
