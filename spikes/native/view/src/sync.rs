//! Settings > Sync (the page's `data/sync.ts`), read only: what a machine
//! shares with the fleet, who has applied which version, and the secrets
//! its tool servers name (names only; a secret's value never leaves its
//! machine).

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use vessel_proto::{ConfigValue, FleetMember, Generation, Manifest, McpTransport};

use crate::Lang;
use crate::ago::Ago;
use crate::machines::StateTone;

/// Where a member stands against the published version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberState {
    /// It has not reported a version.
    NotYet,
    /// It applied the published one (or a later).
    UpToDate,
    /// It is this many versions behind.
    Behind(u64),
}

impl MemberState {
    /// Where `member` stands against `generation`.
    #[must_use]
    pub fn of(member: &FleetMember, generation: Generation) -> Self {
        match member.applied {
            None => Self::NotYet,
            Some(applied) => match generation.get().checked_sub(applied.get()) {
                None | Some(0) => Self::UpToDate,
                Some(behind) => Self::Behind(behind),
            },
        }
    }

    /// Its colour.
    #[must_use]
    pub const fn tone(self) -> StateTone {
        match self {
            Self::NotYet => StateTone::Neutral,
            Self::UpToDate => StateTone::Ok,
            Self::Behind(_) => StateTone::Warn,
        }
    }
}

/// A member's row: the version it applied, its kept edits, its last
/// report.
#[must_use]
pub fn member_line(lang: Lang, member: &FleetMember, now: DateTime<Utc>) -> Vec<String> {
    let words = lang.machines();
    [
        Some(words.applied(member.applied.map(Generation::get))),
        Some(match KeptEdits::of(member) {
            KeptEdits::Unreported => lang.sync().kept(KeptEdits::Unreported),
            KeptEdits::Known(count) => words.kept_edits(count),
        }),
        member
            .reported_at
            .map(|reported| words.reported(Ago::of(reported, now))),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The members with kept edits (a file changed on that machine, kept over
/// the shared one).
#[must_use]
pub fn with_conflicts(members: &[FleetMember]) -> Vec<&FleetMember> {
    members
        .iter()
        .filter(|member| !member.conflicts.is_empty())
        .collect()
}

/// The secrets the shared tool servers name, each once, sorted.
#[must_use]
pub fn secret_names(manifest: &Manifest) -> Vec<String> {
    let names: BTreeSet<String> = manifest
        .mcp
        .values()
        .flat_map(McpTransport::values)
        .filter_map(|value| match value {
            ConfigValue::Secret(name) => Some(name.as_str().to_owned()),
            ConfigValue::Text(_) => None,
        })
        .collect();
    names.into_iter().collect()
}

mod member;
pub use member::{MemberBadge, MemberRow};
mod page;
mod rows;
mod snapshot;
pub use page::{LocalCommand, Page, Part, Problem, Reading};
pub use rows::{FileKind, KeptEdits, connection_variant};
pub use snapshot::{
    FileMode, FileSize, Home, Presence, Published, Reads, Secret, SecretEmpty, SharedFile,
    Snapshot, Usage,
};

impl MemberState {
    /// A short state badge uses the shared semantic vocabulary.
    #[must_use]
    pub const fn variant(self) -> crate::badge::Variant {
        match self {
            Self::NotYet => crate::badge::Variant::Muted,
            Self::UpToDate => crate::badge::Variant::Success,
            Self::Behind(_) => crate::badge::Variant::Warning,
        }
    }
}

/// Read progress is neutral, independently of a member's behind status.
pub const LOADING: crate::badge::Variant = crate::badge::Variant::Muted;
/// The member reported that it preserved its own changes.
pub const KEPT_EDITS_NOTICE: crate::alert::Variant = crate::alert::Variant::Warning;
/// The node's conflict-copy suffix, a path value rather than translated prose.
pub const INCOMING_SUFFIX: &str = ".vessel-incoming";
