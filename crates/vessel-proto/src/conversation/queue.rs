//! Prompts sent while a turn runs (protocol 10, ADR 0024): how a prompt is
//! delivered, and the queue the node keeps for a session.
//!
//! [Decision Log]
//! - Purpose: a person (or an agent) writes the next message while the
//!   agent still works: to wait its turn, to correct the running turn, or
//!   to replace it. All three references offer the three; none sends a
//!   prompt to an agent mid-turn, because the agents disagree on what that
//!   does (queued by Claude's and Codex's adapters, cancelling in Gemini,
//!   refused by `OpenCode`).
//! - Choice: the sender says how ([`Delivery`]). The queue is the node's
//!   (agent-orchestrator and GOAT keep theirs on the server; Paseo's lives
//!   in one browser and is lost with it), part of the session's state:
//!   every client sees it, the store keeps it, and an entry is edited,
//!   moved, removed or sent now by any client ([`QueueChange`]).
//! - It moves one entry per turn (GOAT's way; agent-orchestrator merges
//!   every waiting entry into one turn, which loses where each prompt
//!   ends), and it is held, never emptied, when a person cancels a turn, a
//!   turn fails, or the session ends ([`QueueHold`]): nothing waiting is
//!   sent after a person said stop (GOAT holds in all three cases;
//!   agent-orchestrator drops entries at a cancel).
//! - Bounded ([`PromptQueue::MAX_ENTRIES`], [`PromptQueue::MAX_BYTES`]):
//!   none of the references bounds its queue; every change is logged with
//!   the whole queue, so its size is each such record's size.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agent::ContentBlock;
use crate::id::QueueEntryId;
use crate::profile::Creator;

/// How a prompt goes to a session (`session.prompt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    /// Start a turn now; refused while one runs (what `session.send`'s
    /// prompt has always done).
    Start,
    /// Start a turn when nothing runs and nothing waits; otherwise wait in
    /// the queue.
    Queue,
    /// Add it to the running turn (agents with
    /// [`AgentAbilities::steer`](super::AgentAbilities::steer)); with no
    /// turn running, start one.
    Steer,
    /// Cancel the running turn, wait for it to end, then start this one.
    Interrupt,
}

/// What became of a delivered prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "delivered", rename_all = "snake_case")]
pub enum Delivered {
    /// It started a turn (`turn_started`).
    Started,
    /// It was added to the running turn (`steered`).
    Steered,
    /// It waits in the queue.
    Queued {
        /// Its entry.
        entry: QueueEntryId,
    },
}

/// A prompt waiting for its turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueuedPrompt {
    /// Local canonical notice association; never a request selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<crate::timeline::PromptProvenance>,
    /// The caller's identity follows a queued message into its canonical echo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_message_id: Option<crate::timeline::ClientMessageId>,
    /// The entry's id.
    pub id: QueueEntryId,
    /// What will be sent.
    pub content: Vec<ContentBlock>,
    /// When it was queued (or last edited).
    pub queued_at: DateTime<Utc>,
    /// Raised by each edit; an edit names the revision it was made from,
    /// so two people editing one entry cannot overwrite each other unseen
    /// (agent-orchestrator checks the same).
    #[serde(default)]
    pub revision: u32,
    /// Who queued it.
    #[serde(default)]
    pub by: Creator,
}

/// Why the queue does not move on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum QueueHold {
    /// A person cancelled a turn.
    Cancelled,
    /// A turn failed.
    Failed,
    /// The session ended with prompts waiting.
    Ended,
    /// Someone held it (`session.queue hold`).
    Asked,
}

/// A session's queue.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PromptQueue {
    /// The prompts waiting, next first.
    #[serde(default)]
    pub entries: Vec<QueuedPrompt>,
    /// Why it does not move on its own; absent while it does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held: Option<QueueHold>,
}

impl PromptQueue {
    /// The most entries a queue holds.
    pub const MAX_ENTRIES: usize = 32;
    /// The most content a queue holds, in bytes of its entries' JSON. Each
    /// change logs the whole queue, so this is also the most one
    /// `queue_changed` record weighs.
    pub const MAX_BYTES: usize = 128 * 1024;

    /// Whether nothing waits.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A change to a session's queue (`session.queue`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "change", rename_all = "snake_case")]
pub enum QueueChange {
    /// Replace an entry's content (it keeps its place).
    Edit {
        /// Which entry.
        entry: QueueEntryId,
        /// The revision the edit was made from; another is `conflict`.
        revision: u32,
        /// Its new content.
        content: Vec<ContentBlock>,
    },
    /// Put an entry at another place: before another entry, or last.
    /// Named by entry rather than index, so entries leaving the queue
    /// meanwhile do not move it somewhere else (GOAT's `beforeRunId`).
    Move {
        /// Which entry.
        entry: QueueEntryId,
        /// The entry it goes before; absent is the end.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        before: Option<QueueEntryId>,
    },
    /// Take an entry out.
    Remove {
        /// Which entry.
        entry: QueueEntryId,
    },
    /// Take an entry out and deliver it now.
    SendNow {
        /// Which entry.
        entry: QueueEntryId,
        /// How: `start`, `steer` or `interrupt` (not `queue`).
        delivery: Delivery,
    },
    /// Stop the queue from moving on its own.
    Hold,
    /// Let it move again; between turns, its first entry starts.
    Resume,
    /// Empty it.
    Clear,
}

/// Reply to `session.queue`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueueChanged {
    /// The queue afterwards.
    pub queue: PromptQueue,
    /// For `send_now` and `resume`: what became of the entry sent, if one
    /// was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivered: Option<Delivered>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_and_deliveries_have_their_wire_names() {
        let entry = QueueEntryId::random();
        let change = QueueChange::SendNow {
            entry,
            delivery: Delivery::Steer,
        };
        let json = serde_json::to_string(&change).unwrap();
        assert_eq!(
            json,
            format!(r#"{{"change":"send_now","entry":"{entry}","delivery":"steer"}}"#)
        );
        assert_eq!(serde_json::from_str::<QueueChange>(&json).unwrap(), change);
        assert_eq!(
            serde_json::to_string(&Delivered::Queued { entry }).unwrap(),
            format!(r#"{{"delivered":"queued","entry":"{entry}"}}"#)
        );
        assert_eq!(
            serde_json::to_string(&QueueChange::Hold).unwrap(),
            r#"{"change":"hold"}"#
        );
    }

    #[test]
    fn an_empty_queue_is_empty_json() {
        assert_eq!(
            serde_json::to_string(&PromptQueue::default()).unwrap(),
            r#"{"entries":[]}"#
        );
        let held: PromptQueue = serde_json::from_str(r#"{"held":"cancelled"}"#).unwrap();
        assert_eq!(held.held, Some(QueueHold::Cancelled));
        assert!(held.is_empty());
    }
}
