//! Plain row facts: exact identifiers remain values, never guessed roles.

use super::MemberState;
use crate::{Lang, devices::time::Age};
use chrono::{DateTime, FixedOffset, Utc};
use vessel_proto::{BundlePath, FleetMember, FleetNode};

/// What a shared bundle path represents, without inspecting its contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// The fleet's instructions file.
    Instructions,
    /// A skill or one of its supporting files.
    Skill,
    /// Another bundle path the home publishes.
    Other,
}
impl FileKind {
    /// The manifest's canonical names; no filesystem I/O.
    #[must_use]
    pub fn of(path: &BundlePath) -> Self {
        match path.as_str() {
            "instructions.md" | "AGENTS.md" => Self::Instructions,
            path if path.starts_with("skills/") => Self::Skill,
            _ => Self::Other,
        }
    }
}

/// Full history beneath a member's short state badge. Reports describe
/// what its home last heard, never a guessed current connection or sync.
#[must_use]
pub fn member_details(
    lang: Lang,
    member: &FleetMember,
    state: Option<MemberState>,
    now: DateTime<Utc>,
    offset: FixedOffset,
) -> String {
    let words = lang.sync();
    words
        .comparison(state)
        .into_iter()
        .chain([
            words.applied(member.applied),
            words.kept(member.conflicts.len()),
            words.reported(member.reported_at.map(|at| Age::of(at, now, offset))),
        ])
        .collect::<Vec<_>>()
        .join("\n")
}

/// A connection's badge meaning is independent of a member's last report.
#[must_use]
pub const fn connection_variant(node: &FleetNode) -> crate::badge::Variant {
    if node.reachable {
        crate::badge::Variant::Success
    } else {
        crate::badge::Variant::Error
    }
}

/// Member comparisons stay unknown when the publication could not be read.
#[must_use]
pub const fn member_variant(state: Option<MemberState>) -> crate::badge::Variant {
    match state {
        Some(state) => state.variant(),
        None => crate::badge::Variant::Muted,
    }
}
