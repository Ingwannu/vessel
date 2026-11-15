//! One window's resource admission and asynchronous reply fences. This module
//! owns no text fields, transcripts, connection cache or backend lifecycle.
//! Resource owners keep concrete snapshots behind the app's scoped accessors.
mod identity;
mod registry;
mod retirement;
pub use retirement::{FollowRetirement, RetiredOperation};
mod transport;
pub use identity::{
    FollowBinding, FollowGeneration, HoldReason, OperationKind, OperationTicket, OwnerReadiness,
    PanelClaim, PanelFocus, RegistryId, ResidentEpoch, ResidentProblem, ResidentToken,
    ResourceOwner, SessionAddress, SessionContext, Visibility,
};
pub use registry::Residents;

/// Ordinary per-pane warm target, matching the reference panel host.
pub const PANE_RESIDENT_TARGET: usize = 3;
/// Admission bound per window; protected input is never evicted for capacity.
pub const MAX_RESIDENT_SESSIONS: usize = 64;
/// Existing inactive-workspace retention interval reused for clean residents.
pub const UNCLAIMED_RESIDENT_TTL: std::time::Duration = std::time::Duration::from_secs(600);
/// Finite attach/read safety deadline, separate from backend session lifetime.
pub const RESIDENT_ATTACH_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);
/// Bounded in-flight admissions; refusal preserves all existing operations.
pub const MAX_RESIDENT_OPERATIONS: usize = 64;
#[cfg(test)]
mod tests;
