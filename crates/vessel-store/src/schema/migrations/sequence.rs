//! Canonical append order; SQL bodies and relative version constants stay in parent.
//!
//! [Decision Log]
//! - Purpose: keep future append-only registrations out of the SQL body's size budget.
//! - Choice: move the existing array, without an allocator, new registry or SQL change.
//! - Invariant: integration appends here; the parent re-export preserves schema callers.
use super::{
    BACKGROUND_SCHEMA, INBOX, LABELS, SIDE_CONTEXT, SUBAGENT_RELATIONS, TITLE_ORIGIN, USAGE_LEDGER,
    V1, V2, V3, V4, V5, V6, V7, V8, V9, V10, V11, V12, V13,
};

/// Every migration, oldest first. Index `n` takes the schema from version
/// `n` to `n + 1`.
pub(in crate::schema) const MIGRATIONS: &[&str; BACKGROUND_SCHEMA.saturating_add(1)] = &[
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    V10,
    V11,
    V12,
    V13,
    SIDE_CONTEXT,
    USAGE_LEDGER,
    INBOX,
    LABELS,
    TITLE_ORIGIN,
    SUBAGENT_RELATIONS,
    super::super::workspace_registry::SQL,
    super::super::listing::SQL,
    crate::notification_claims::SCHEMA,
];
