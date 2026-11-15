//! Creation receipts and resource claims share the database's transaction authority.

/// Integration repoints this one assignment when another package lands first.
pub(super) const RECEIPTS_SCHEMA: usize = super::migrations::REGISTRY_SCHEMA.saturating_add(1);
pub(super) const OWNER_SCHEMA: usize = RECEIPTS_SCHEMA.saturating_add(1);

pub(super) const CREATION: &str = "
CREATE TABLE terminal_profile_settings (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK (singleton=1),
    revision INTEGER NOT NULL CHECK (revision>=0),
    profiles TEXT NOT NULL
);
CREATE TABLE creation_receipts (
    key_hash TEXT PRIMARY KEY NOT NULL,
    operation TEXT UNIQUE NOT NULL,
    reserved_bytes INTEGER NOT NULL,
    receipt TEXT NOT NULL
);
CREATE TABLE creation_claims (
    kind TEXT NOT NULL,
    resource TEXT NOT NULL,
    operation TEXT NOT NULL,
    PRIMARY KEY(kind,resource)
);
";

/// Existing rows remain person-owned without rewriting their historical key hash.
pub(super) const OWNER: &str =
    r#"ALTER TABLE creation_receipts ADD COLUMN owner TEXT NOT NULL DEFAULT '{"kind":"person"}';"#;
