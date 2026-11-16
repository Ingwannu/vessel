//! Identifier and counter newtypes.
//!
//! Every identifier that crosses the wire has its own type so the compiler
//! stops a `SessionId` from being passed where a `NodeId` is expected. There
//! are three shapes:
//!
//! - node-minted random ids (`NodeId`, `SessionId`, `PermissionId`): `UUIDv4`;
//! - harness-minted ids we only carry (`HarnessSessionId`, `ToolCallId`, ...):
//!   opaque strings, compared but never parsed;
//! - per-connection or per-session counters (`RequestId`, `SubscriptionId`,
//!   `EventSeq`): `u64`, advanced only through checked methods.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Declares a `UUIDv4` identifier minted by the node.
macro_rules! uuid_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Mint a fresh random id.
            #[must_use]
            pub fn random() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s.trim()).map(Self)
            }
        }
    };
}

/// Declares an opaque identifier minted by something other than the node.
///
/// Backed by `Arc<str>`: these ids are copied into many events and fanned
/// out to many subscribers, so cloning must not allocate.
macro_rules! opaque_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(transparent)]
        pub struct $name(Arc<str>);

        impl $name {
            /// Wrap an id received from elsewhere.
            #[must_use]
            pub fn new(id: impl Into<Arc<str>>) -> Self {
                Self(id.into())
            }

            /// The id as received.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

/// Declares a monotonically increasing `u64` counter.
macro_rules! counter_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            /// The first value.
            pub const ZERO: Self = Self(0);

            /// Wrap a raw value (for decoding and tests).
            #[must_use]
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            /// The raw value.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }

            /// The value after this one, or `None` once `u64` is exhausted.
            ///
            /// Callers treat `None` as "this stream is full" and end it,
            /// rather than wrapping around and reusing ids.
            #[must_use]
            pub const fn next(self) -> Option<Self> {
                match self.0.checked_add(1) {
                    Some(value) => Some(Self(value)),
                    None => None,
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

uuid_id! {
    /// Node-minted finish intent identity; never a public submission namespace.
    FinishIntentId
}

uuid_id! {
    /// Correlation for one parent execution; its spelling never grants admission.
    ExecutionIncarnationId
}

uuid_id! {
    /// Independent workspace identity; shared cwd never implies shared ownership.
    WorkspaceId
}

uuid_id! {
    /// Caller-minted receipt identity for retry-safe workspace creation.
    WorkspaceOperationId
}

uuid_id! {
    /// Identity of a Vessel node; one per machine, stable across restarts.
    NodeId
}

uuid_id! {
    /// A session on a node. Unique across the fleet, so it can be used
    /// without the node id once peers exist.
    SessionId
}

uuid_id! {
    /// A pending permission request inside a structured session. Minted by the
    /// node (not the harness) so many clients can answer the same request and
    /// the first answer wins.
    PermissionId
}

uuid_id! {
    /// A paired device: one client machine holding its own credential for a
    /// node (ADR 0010). Minted by the node at pairing.
    DeviceId
}

uuid_id! {
    /// An open transfer: a snapshot of files a node holds for whoever opened
    /// it (protocol 6). Random, but not a secret: the node also checks that
    /// the caller reading or closing it is the one that opened it.
    TransferId
}

uuid_id! {
    /// An install a node started (`agent.install`, protocol 11, ADR 0032):
    /// minted by the node, kept until the next install or a restart.
    InstallId
}

uuid_id! {
    /// A prompt waiting in a session's queue (protocol 10), minted by the
    /// node when it is queued.
    QueueEntryId
}

uuid_id! {
    /// A question an agent asks (protocol 10). Minted by the node, as a
    /// permission request's id is, so many clients can answer and the first
    /// answer wins.
    QuestionId
}

uuid_id! {
    /// A file sent with a prompt, kept with its session (protocol 10).
    AttachmentId
}

uuid_id! {
    /// An upload in progress (protocol 10). Random, but not a secret: only
    /// the caller that began it may write to it or finish it.
    UploadId
}

uuid_id! {
    /// A project: a folder the node works in (protocol 10, ADR 0027).
    ProjectId
}

uuid_id! {
    /// A worktree the node made for a project (protocol 10, ADR 0027).
    WorktreeId
}

uuid_id! {
    /// A terminal beside a session: a shell, or a run of a project's script
    /// (protocol 11, ADR 0031).
    TerminalId
}

uuid_id! {
    /// A schedule (protocol 11, ADR 0033).
    ScheduleId
}

uuid_id! {
    /// A saved prompt (protocol 11, ADR 0033).
    SavedPromptId
}

uuid_id! {
    /// A webhook (protocol 11, ADR 0034). Random, and in the hook's path,
    /// but not a secret: every request is signed with the hook's own.
    HookId
}

opaque_id! {
    /// The harness's own session id (an ACP `sessionId`). Kept so a
    /// structured session can be resumed by the harness after a node restart.
    HarnessSessionId
}

opaque_id! {
    /// A tool call id assigned by the harness.
    ToolCallId
}

opaque_id! {
    /// The id an agent gives one of its messages (ACP's `messageId` on a
    /// message chunk): what a fork at a turn names (protocol 14, ADR 0052).
    /// At most [`AgentMessageId::MAX_BYTES`]; the node drops a longer one.
    AgentMessageId
}

impl AgentMessageId {
    /// The longest id kept.
    pub const MAX_BYTES: usize = 256;
}

opaque_id! {
    /// A permission option id offered by the harness (for example
    /// `allow-once`). Echoed back verbatim when a client picks it.
    PermissionOptionId
}

opaque_id! {
    /// A model name as the harness understands it (`opus`, `gpt-5`, ...).
    /// The set is open and harness-specific, so it is not an enum.
    ModelId
}

opaque_id! {
    /// A session config option's id as the agent names it (ACP's
    /// `configId`, for example `model` or `mode`; protocol 9). Echoed back
    /// verbatim when a client changes the option.
    OptionId
}

opaque_id! {
    /// One value of a select option, as the agent names it (ACP's `value`;
    /// protocol 9). Echoed back verbatim when a client picks it.
    OptionValueId
}

counter_id! {
    /// Client-assigned id correlating a request with its response; unique
    /// per connection.
    RequestId
}

counter_id! {
    /// Node-assigned id of one event subscription; unique per connection.
    SubscriptionId
}

counter_id! {
    /// Position of an event in one session's log. Starts at zero and has no
    /// gaps, so a client resumes with "everything from seq N".
    EventSeq
}

uuid_id! {
    /// A person's transient spoken-conversation lease (ADR 0080).
    VoiceId
}

uuid_id! {
    /// A node-minted generation; replacing a lease never reuses authority.
    VoiceEpoch
}

uuid_id! {
    /// A physical playback receipt, separate from its speech group.
    PlaybackId
}

uuid_id! {
    /// One ordered spoken utterance, without persisting its PCM.
    SpeechGroupId
}

counter_id! {
    /// Contiguous microphone input order, never a session event sequence.
    VoiceSeq
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_stop_instead_of_wrapping() {
        assert_eq!(EventSeq::ZERO.next(), Some(EventSeq::new(1)));
        assert_eq!(EventSeq::new(u64::MAX).next(), None);
    }

    #[test]
    fn uuid_ids_round_trip_through_text() {
        let id = SessionId::random();
        assert_eq!(id.to_string().parse::<SessionId>().unwrap(), id);
    }

    #[test]
    fn opaque_ids_serialise_as_plain_strings() {
        let id = ToolCallId::new("toolu_01");
        assert_eq!(serde_json::to_string(&id).unwrap(), r#""toolu_01""#);
    }
}

#[cfg(test)]
mod workspace_identity_tests {
    use super::{SessionId, WorkspaceId, WorkspaceOperationId, WorktreeId};
    #[test]
    fn workspace_identity_round_trips_through_serde_and_text() {
        let id = WorkspaceId::random();
        assert_eq!(id.to_string().parse::<WorkspaceId>().unwrap(), id);
        assert_eq!(
            serde_json::from_str::<WorkspaceId>(&serde_json::to_string(&id).unwrap()).unwrap(),
            id
        );
        assert!(serde_json::from_str::<WorkspaceId>("\"invalid\"").is_err());
        assert_ne!(
            std::any::TypeId::of::<WorkspaceId>(),
            std::any::TypeId::of::<WorktreeId>()
        );
        assert_ne!(
            std::any::TypeId::of::<WorkspaceId>(),
            std::any::TypeId::of::<SessionId>()
        );
    }
    #[test]
    fn workspace_operation_identity_is_distinct_and_validated_at_decode() {
        let id = WorkspaceOperationId::random();
        assert_eq!(id.to_string().parse::<WorkspaceOperationId>().unwrap(), id);
        assert_eq!(
            serde_json::from_str::<WorkspaceOperationId>(&serde_json::to_string(&id).unwrap())
                .unwrap(),
            id
        );
        assert!(serde_json::from_str::<WorkspaceOperationId>("\"invalid\"").is_err());
        assert_ne!(
            std::any::TypeId::of::<WorkspaceOperationId>(),
            std::any::TypeId::of::<WorkspaceId>()
        );
    }
}

uuid_id! {
    /// One setup invocation, independent of worktree path and node restart.
    SetupRunId
}

counter_id! {
    /// Snapshot revision within a single setup invocation.
    SetupRevision
}
