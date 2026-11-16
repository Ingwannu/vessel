//! Dormant keyed acceptance values shared by prompts and native commands.
//! Legacy Delivery/Delivered and turn tickets do not acquire command semantics.
use crate::native_commands::NativeCommandStatus;
use crate::{EventSeq, QueueEntryId};
use serde::{Deserialize, Serialize};

/// The actor selects behavior atomically with its held foreground turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SubmissionDelivery {
    /// Start only when no foreground turn runs.
    Start,
    /// Retain in the node queue until delivery can start.
    Queue,
    /// Join an active turn only if the provider accepts steering.
    Steer,
    /// Cancel the held active turn before starting this prompt.
    Interrupt,
    /// Steer when offered; otherwise cancel that same held turn and start once.
    /// An uncertain steer error never authorizes interruption or another send.
    SteerOrInterrupt,
}

/// What actually happened; never guessed from the requested policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "disposition", rename_all = "snake_case")]
pub enum PromptDisposition {
    /// The provider accepted a new prompt.
    Started,
    /// The provider accepted this message into its current turn.
    Steered,
    /// The exact entry is committed in the durable node queue.
    Queued {
        /// Durable queue identity retained on replay.
        entry: QueueEntryId,
    },
    /// A real cancellation completed before the replacement prompt was accepted.
    InterruptedAndStarted {
        /// Canonical start record of the turn that was interrupted.
        interrupted: EventSeq,
    },
}

/// A command acceptance never creates a model turn or increments busy counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubmissionAcceptedKind {
    /// Actual prompt delivery, independent of the requested policy.
    Prompt {
        /// The serialized actor's result.
        disposition: PromptDisposition,
    },
    /// Provider-native command acceptance and its canonical committed echo.
    Command {
        /// Localizable result; derives its kind without repeating the objective.
        status: NativeCommandStatus,
        /// Sequence of the committed `CommandAccepted` event.
        echo: EventSeq,
    },
}

/// Provider refusal is safe only before a possibly mutating RPC is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum NativeCommandRefusal {
    /// This provider transport has no implementation.
    Unsupported,
    /// The current run did not advertise this exact native kind.
    NotOffered,
    /// Actual turn or provider state cannot accept this operation.
    InvalidState,
    /// Actual transport version is absent or below its required floor.
    VersionUnavailable,
    /// Goal launch flags or matched autonomous lifecycle support are absent.
    GoalsNotLaunched,
}

/// A stable refusal guarantees that neither cancellation nor send occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum SubmissionRefusal {
    /// The owning session ended before delivery.
    SessionEnded,
    /// The current driver/session cannot accept the operation.
    Unavailable,
    /// Start-only delivery met an active turn.
    TurnRunning,
    /// Content validation refused before provider work.
    InvalidContent,
    /// Attachments could not be loaded before any cancellation or send.
    AttachmentsUnavailable,
    /// The finite queue refused admission before insertion.
    QueueFull,
    /// Session ownership was moving before delivery.
    Moving,
    /// Provider-native command admission refused before dispatch.
    NativeCommand {
        /// Closed reason without raw RPC, stderr or objective text.
        cause: NativeCommandRefusal,
    },
}
