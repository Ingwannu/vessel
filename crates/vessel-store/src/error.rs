//! The store's error type.

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use vessel_proto::{DeviceId, ErrorCode, EventSeq, NodeId, SessionId};

use crate::attachments::files::AttachmentFileError;

/// A value that does not fit the on-disk representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    /// A sequence number above `i64::MAX`.
    Seq,
    /// A timestamp outside 1677..2262 (nanoseconds in an `i64`).
    Time,
    /// A byte count above `i64::MAX`.
    Bytes,
}

/// A stored column whose value failed to decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    /// `sessions.spec` (a `SessionSpec`).
    Spec,
    /// `sessions.status` (a `SessionStatus`).
    Status,
    /// `events.event` (a `SessionEvent`).
    Event,
    /// `sessions.profile` (a `ProfileName`, schema 3).
    Profile,
    /// `sessions.created_by` (a `Creator`, schema 3).
    CreatedBy,
    /// `sessions.via` (a `NodeId`, schema 4).
    Via,
    /// `sessions.origin` (a `SessionOrigin`, schema 4).
    Origin,
    /// `sessions.model` (a `SessionModel`, schema 5).
    Model,
    /// `sessions.title` (a `SessionTitle`, schema 6).
    Title,
    /// `sessions.choices` (`OptionChoice`s, schema 6).
    Choices,
    /// `sessions.abilities` (`AgentAbilities`, schema 7).
    Abilities,
    /// `sessions.usage` (`ContextUsage`, schema 7).
    Usage,
    /// Durable inbox item or tool projection.
    Inbox,
    /// `sessions.queue` (`PromptQueue`, schema 7).
    Queue,
    /// The bounded latest plan (schema 14, ADR 0045).
    LatestPlan,
    /// `sessions.labels` (a `SessionLabels`, the labels migration, ADR 0063).
    Labels,
    /// `sessions.title_origin` (a `TitleOrigin`, ADR 0062).
    TitleOrigin,
    /// Durable managed-child detach receipt (ADR 0090).
    DetachReceipt,
    /// `sessions.listing` (a `SessionListing`, ADR 0063).
    Listing,
}

/// Anything that can go wrong in the store.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Semantic label refusals remain structured at the RPC boundary.
    #[error("workspace label operation was refused ({0:?})")]
    WorkspaceLabelRefused(#[source] vessel_proto::workspace_labels::WorkspaceLabelFailure),
    /// A persisted label name no longer satisfies the validated grammar.
    #[error("stored workspace label name is invalid")]
    WorkspaceLabelName(#[source] vessel_proto::workspace_labels::LabelNameError),
    /// A stored color cannot be decoded; its raw data is never displayed.
    #[error("stored workspace label color is invalid")]
    WorkspaceLabelColor(#[source] serde_json::Error),
    /// A stored normalized key disagrees with the definition or a join is missing.
    #[error("stored workspace label identity is inconsistent")]
    WorkspaceLabelIdentity,
    /// A `SQLite` count did not fit the checked public count type.
    #[error("workspace label count is outside its representation")]
    WorkspaceLabelCount(#[source] std::num::TryFromIntError),
    /// Appending must never wrap and reorder an existing assignment.
    #[error("workspace label assignment order is exhausted")]
    WorkspaceLabelOrder,
    /// A file in the data directory could not be read or written.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file or directory.
        path: PathBuf,
        /// What failed.
        #[source]
        source: std::io::Error,
    },
    /// The data directory is accessible to other users and is not ours to
    /// change.
    #[error(
        "{} is accessible to other users and cannot be made owner-only (is it another \
         user's?); fix its permissions or choose another data directory",
        path.display()
    )]
    NotPrivate {
        /// The directory.
        path: PathBuf,
        /// Why its mode could not be changed.
        #[source]
        source: std::io::Error,
    },
    /// Another node holds the data directory's lock.
    #[error(
        "{} is held by another node{}",
        path.display(),
        holder.as_deref().map(|owner| format!(" (pid {}, version {})", owner.pid, owner.version)).unwrap_or_default()
    )]
    Locked {
        /// The lock file.
        path: PathBuf,
        /// That node's record, when it could be read.
        holder: Option<Box<crate::lock::NodeOwner>>,
    },
    /// The operating system's random source failed.
    #[error("no randomness available")]
    Random(#[source] getrandom::Error),
    /// `node.id` does not hold a node id.
    #[error("{} does not hold a node id", path.display())]
    InvalidNodeId {
        /// The file.
        path: PathBuf,
        /// Why it did not parse.
        #[source]
        source: <NodeId as FromStr>::Err,
    },
    /// `node.key` does not hold a key. It is never replaced automatically:
    /// a new key would unpair every device (ADR 0009).
    #[error(
        "{} does not hold a node key (expected 32 bytes, found {len}); restore it from a \
         backup, or delete it to make a new key (every paired device must then pair again)",
        path.display()
    )]
    InvalidNodeKey {
        /// The file.
        path: PathBuf,
        /// Its size.
        len: usize,
    },
    /// The token file does not hold a token this node could have written.
    #[error("{} does not hold a valid token", path.display())]
    InvalidToken {
        /// The file.
        path: PathBuf,
    },
    /// The database is from a newer Vessel; refusing to touch it.
    #[error("database schema version {found} is newer than this build supports ({supported})")]
    NewerSchema {
        /// Version found on disk.
        found: i64,
        /// Newest version this build knows.
        supported: i64,
    },
    /// `SQLite` failed.
    #[error("database error")]
    Sqlite(#[from] rusqlite::Error),
    /// Committing a batch failed, so every operation in it failed. Shared
    /// because one failure answers several callers.
    #[error("committing to the database failed")]
    Commit(#[source] Arc<rusqlite::Error>),
    /// Missing and inactive are distinct admission refusals; callers never parse a generic message.
    #[error("workspace {0} is not registered")]
    RegistryMissing(vessel_proto::WorkspaceId),
    /// Existing ownership is retained, but new actors/assignments require an Active lifecycle.
    #[error("workspace {0} is not active")]
    RegistryInactive(vessel_proto::WorkspaceId),
    /// A lifecycle or membership CAS failed; no partial mutation was committed.
    #[error("workspace registry revision changed or the workspace is not active")]
    RegistryConflict,
    /// Retained ownership/identity capacity is reserved before any new side effect.
    #[error("workspace registry retained capacity is full")]
    RegistryFull,
    /// Per-record logical bytes exceed their pre-reserved worst-case allowance.
    #[error("workspace registry record exceeds its retained byte allowance")]
    RegistryRecordTooLarge,
    /// A registry counter must never wrap or silently lose ordering.
    #[error("workspace registry revision is exhausted")]
    RegistryRevisionExhausted,
    /// Signed SQL quantities are converted without truncating or discarding their cause.
    #[error("registry revision is outside the SQLite storage range")]
    RegistryRevisionRange(#[source] std::num::TryFromIntError),
    /// Typed registry state corruption is explicit; rows are never silently skipped.
    #[error("stored workspace registry data is invalid")]
    RegistryDecode(#[source] serde_json::Error),
    /// A stored value did not decode; the database was edited or corrupted.
    #[error("stored {column:?} of session {session} is invalid")]
    Corrupt {
        /// Session the row belongs to.
        session: SessionId,
        /// Column that failed.
        column: Column,
        /// Why.
        #[source]
        source: serde_json::Error,
    },
    /// A stored session id did not parse.
    #[error("stored session id {text:?} is invalid")]
    CorruptId {
        /// The stored text.
        text: String,
        /// Why.
        #[source]
        source: <SessionId as FromStr>::Err,
    },
    /// A stored device row did not decode; the database was edited or
    /// corrupted.
    #[error("stored device {id:?} has an invalid {what}")]
    CorruptDevice {
        /// The stored id text.
        id: String,
        /// Which column.
        what: &'static str,
    },
    /// A stored attachment row did not decode; the database was edited or
    /// corrupted.
    #[error("stored attachment {id:?} has an invalid {what}")]
    CorruptAttachment {
        /// The stored id text.
        id: String,
        /// Which column.
        what: &'static str,
    },
    /// A stored message of the search index did not decode (protocol 12);
    /// the database was edited or corrupted.
    #[error("stored message {id} of the search index has an invalid {what}")]
    CorruptMessage {
        /// The piece's id.
        id: i64,
        /// Which column.
        what: &'static str,
    },
    /// No device with this id is stored.
    #[error("device {0} is not known to this node")]
    UnknownDevice(DeviceId),
    /// A device with this id is already stored.
    #[error("device {0} already exists")]
    DeviceExists(DeviceId),
    /// A value could not be encoded for storage.
    #[error("could not encode a value for storage")]
    Encode(#[source] serde_json::Error),
    /// A number does not fit its column.
    #[error("{0:?} out of the storable range")]
    OutOfRange(Quantity),
    /// No session with this id is stored.
    #[error("session {0} is not in the store")]
    UnknownSession(SessionId),
    /// A session with this id is already stored.
    #[error("session {0} is already in the store")]
    SessionExists(SessionId),
    /// The session is still live in the store (its log is not complete),
    /// so it can neither be deleted nor started again.
    #[error("session {0} is still live (its log is not complete)")]
    SessionLive(SessionId),
    /// The session has ended, so it keeps no new attachment (protocol 10).
    #[error("session {0} has ended; it takes no more attachments")]
    SessionEnded(SessionId),
    /// A kept attachment's file is missing or not the one kept, or a
    /// finished upload could not be put in place (protocol 10).
    #[error(transparent)]
    Attachment(#[from] AttachmentFileError),
    /// Records must be appended gap-free, in order.
    #[error("session {session}: expected record {expected}, got {got}")]
    OutOfOrder {
        /// The session.
        session: SessionId,
        /// The sequence number the log needed next.
        expected: EventSeq,
        /// The one it was given.
        got: EventSeq,
    },
    /// An inbox entry was answered or removed before it could be marked.
    #[error("this inbox entry has moved on; refresh the inbox before trying again")]
    InboxGone,
    /// A push lease cannot be registered after revocation or without read.
    #[error("this device cannot register push; pair it again with read access")]
    PushDenied,
    /// Bounded device subscriptions, independent of the number of sessions.
    #[error("64 devices already receive push; unsubscribe an unused device first")]
    PushFull,
    /// A typed stored subscription failed to decode, without printing its secret.
    #[error("stored push subscription is corrupt; subscribe this device again")]
    CorruptPush {
        /// Device whose row needs renewal.
        device: DeviceId,
        /// Decoder failure; never contains the subscription text.
        #[source]
        source: serde_json::Error,
    },
    /// A human cannot detach a session that was never an agent's child.
    #[error("session {0} was never a managed child")]
    NotChild(SessionId),
    /// A stored receipt contradicts immutable creator provenance.
    #[error("session {0} has corrupt parentage")]
    InvalidParentage(SessionId),
    /// Stored mutation time is corrupt, or strict advancement is exhausted.
    #[error("session {session} mutation time cannot be kept: {source}")]
    MutationTime {
        /// The row whose mutation must be refused atomically.
        session: SessionId,
        /// Preserve the typed precision/range/exhaustion reason, without data.
        #[source]
        source: vessel_proto::InvalidSessionUpdatedAt,
    },
    /// The store was closed, or its writer thread is gone.
    #[error("the store is closed")]
    Closed,
    /// The writer thread could not be started.
    #[error("could not start the store writer thread")]
    Thread(#[source] std::io::Error),
    /// The node already has its maximum queued or running usage scans.
    #[error("usage is being read on this machine; try again shortly")]
    UsageBusy(#[source] tokio::sync::TryAcquireError),
    /// A blocking read task failed.
    #[error("a store read task failed")]
    Task(#[source] tokio::task::JoinError),
}

impl StoreError {
    /// The wire error code for this failure.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::WorkspaceLabelRefused(reason) => reason.code(),
            // A row without its file: removed under the node.
            Self::PushDenied => ErrorCode::Forbidden,
            Self::InboxGone
            | Self::RegistryMissing(_)
            | Self::UnknownSession(_)
            | Self::UnknownDevice(_)
            | Self::Attachment(AttachmentFileError::Missing(_)) => ErrorCode::NotFound,
            Self::NotChild(_)
            | Self::RegistryFull
            | Self::RegistryInactive(_)
            | Self::RegistryRecordTooLarge
            | Self::RegistryConflict
            | Self::PushFull
            | Self::SessionExists(_)
            | Self::SessionLive(_)
            | Self::DeviceExists(_)
            | Self::SessionEnded(_)
            | Self::Attachment(AttachmentFileError::Changed { .. })
            | Self::OutOfOrder { .. } => ErrorCode::Conflict,

            Self::Closed | Self::UsageBusy(_) => ErrorCode::Unavailable,
            Self::WorkspaceLabelName(_)
            | Self::WorkspaceLabelColor(_)
            | Self::WorkspaceLabelIdentity
            | Self::WorkspaceLabelCount(_)
            | Self::WorkspaceLabelOrder
            | Self::RegistryRevisionRange(_)
            | Self::RegistryRevisionExhausted
            | Self::RegistryDecode(_)
            | Self::Io { .. }
            | Self::NotPrivate { .. }
            | Self::Locked { .. }
            | Self::Random(_)
            | Self::InvalidNodeId { .. }
            | Self::InvalidToken { .. }
            | Self::InvalidNodeKey { .. }
            | Self::NewerSchema { .. }
            | Self::Sqlite(_)
            | Self::Commit(_)
            | Self::MutationTime { .. }
            | Self::InvalidParentage(_)
            | Self::Corrupt { .. }
            | Self::CorruptPush { .. }
            | Self::CorruptId { .. }
            | Self::CorruptDevice { .. }
            | Self::CorruptAttachment { .. }
            | Self::CorruptMessage { .. }
            | Self::Attachment(AttachmentFileError::Io { .. })
            | Self::Encode(_)
            | Self::OutOfRange(_)
            | Self::Thread(_)
            | Self::Task(_) => ErrorCode::Internal,
        }
    }

    pub(crate) fn io(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> Self {
        let path = path.into();
        move |source| Self::Io { path, source }
    }
}
