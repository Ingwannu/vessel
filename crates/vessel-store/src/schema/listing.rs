//! Creation-only session discovery (ADR 0063): each session's effective
//! listing, a column of its own beside the creation specification.
//!
//! Kept out of `migrations` so that file stays within the size budget, as
//! `workspace_registry` is; released SQL is append-only.

/// Existing rows remain shown without rewriting their creation specifications.
pub(super) const SQL: &str =
    r#"ALTER TABLE sessions ADD COLUMN listing TEXT NOT NULL DEFAULT '"shown"';"#;
