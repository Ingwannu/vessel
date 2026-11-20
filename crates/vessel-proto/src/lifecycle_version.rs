//! Protocol floor for the lifecycle additions in ADR 0063.
//! Kept apart from individual request and metadata types so integration
//! assigns one number without turning those modules into feature buckets.

/// First protocol that carries the lifecycle additions.
///
/// The integrator assigns this number and raises `PROTOCOL_VERSION` at
/// merge. Keeping the base value here avoids claiming a sibling's number.
pub const SINCE: u32 = 20;
