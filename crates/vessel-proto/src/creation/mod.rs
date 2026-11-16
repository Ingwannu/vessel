//! Durable creation identities, reservations and safe snapshots (ADR 0091).
//! No prompt, environment, command template or credential enters a snapshot.

mod receipt;
mod text;

pub use receipt::{
    Advance, Certainty, FirstInput, Kind, Phase, Receipt, Reservation, Resource, Snapshot, Stage,
    WorkspaceClaim,
};
pub use text::{Fingerprint, Key, Revision};

/// Whole-package version; the integrator assigns it after all consumers land.
pub const CREATION_PROTOCOL: Option<crate::ProtocolVersion> = None;

/// Refuse further admission rather than silently expiring an idempotency key.
pub const MAX_RECEIPTS: usize = 10_000;
/// Largest serialized safe receipt, checked on every persisted revision.
pub const MAX_RECEIPT_BYTES: usize = 4 * 1024;
/// Worst-case receipt growth plus fixed indexes and at most two resource claims.
pub const RESERVED_BYTES_PER_RECEIPT: usize = 8 * 1024;
/// Retained logical bytes, excluding SQLite/WAL/filesystem overhead.
pub const MAX_RETAINED_BYTES: usize = 64 * 1024 * 1024;

/// Input or lifecycle errors never echo rejected keys or command/prompt text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CreationError {
    /// Idempotency identities must contain at least one character.
    #[error("creation key cannot be empty")]
    EmptyKey,
    /// A finite observation cannot occupy a call beyond its declared budget.
    #[error("creation observation wait is too long")]
    WaitTooLong,
    /// A bounded identity prevents admission/storage resource exhaustion.
    #[error("creation key exceeds {} Unicode scalars", Key::MAX_CHARS)]
    KeyTooLong,
    /// Keep keys unambiguous in JSON, storage and diagnostic boundaries.
    #[error("creation key cannot contain control characters")]
    KeyControl,
    /// An old intent cannot reuse another request's reserved resources.
    #[error("creation key is already bound to a different intent")]
    IntentConflict,
    /// A resource id cannot be owned by two creation operations.
    #[error("creation resource is already claimed")]
    ResourceClaimed,
    /// A stale caller must observe before attempting another transition.
    #[error("creation revision changed")]
    StaleRevision,
    /// Replaying an invalid transition could repeat an external side effect.
    #[error("invalid creation transition")]
    InvalidTransition,
    /// Never wrap a revision into an older observation.
    #[error("creation revision exhausted")]
    RevisionExhausted,
    /// Receipt and claims are retained; admission fails instead of evicting keys.
    #[error("creation receipt retention capacity reached")]
    Full,
    /// A receipt cannot consume another operation's reserved growth allowance.
    #[error("creation receipt exceeds {MAX_RECEIPT_BYTES} bytes")]
    ReceiptTooLarge,
    /// Damaged reservation accounting fails closed before any effects.
    #[error("creation capacity accounting is invalid")]
    CapacityCorrupt,
    /// An owner/index mismatch cannot be reconciled as an absent receipt.
    #[error("creation receipt namespace metadata is invalid")]
    NamespaceCorrupt,
    /// A receipt cannot proceed after its permanent claims were altered.
    #[error("creation receipt claims are invalid")]
    ClaimsCorrupt,
    /// Observation of a missing receipt is distinct from uncertain work.
    #[error("creation receipt not found")]
    NotFound,
}

impl CreationError {
    /// One mapping for clients, store and service refusal projection.
    #[must_use]
    pub const fn code(self) -> crate::ErrorCode {
        match self {
            Self::EmptyKey | Self::KeyTooLong | Self::KeyControl | Self::WaitTooLong => {
                crate::ErrorCode::InvalidRequest
            }
            Self::IntentConflict
            | Self::ResourceClaimed
            | Self::StaleRevision
            | Self::InvalidTransition => crate::ErrorCode::Conflict,
            Self::RevisionExhausted
            | Self::ReceiptTooLarge
            | Self::CapacityCorrupt
            | Self::NamespaceCorrupt
            | Self::ClaimsCorrupt => crate::ErrorCode::Internal,
            Self::Full => crate::ErrorCode::Unavailable,
            Self::NotFound => crate::ErrorCode::NotFound,
        }
    }
}
