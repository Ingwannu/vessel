//! A member is one hint and one badge, as Paseo's settings rows. Conflict
//! paths belong only to the following notice, never repeated as row prose.
//!
//! [Decision Log]
//! - Purpose: preserve what was reported without four stacked explanations.
//! - Constraint: an empty conflict list can be a missing-report default;
//!   a partial report can have a time or paths but no applied version.
//! - Alternatives: hide all partial facts, or say "not reported" beside a
//!   known report time. Both would misstate what the node actually sent.
//! - Choice: one compact metadata hint and a typed badge distinguishing
//!   wholly missing reports from reports without a version; edits are in
//!   the separate notice. Why: one signal per fact, no inferred zero.
//! - Cost: exact times remain on hover; counts are represented by the paths.
use super::MemberState;
use crate::{Lang, badge::Variant, devices::time::Age};
use chrono::{DateTime, FixedOffset, Utc};
use vessel_proto::FleetMember;

/// The state a member's short badge actually establishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberBadge {
    /// No version, time or conflict paths were reported.
    Unreported,
    /// Some report facts exist, but no applied version was supplied.
    VersionUnreported,
    /// The member applied the current shared version, or a later one.
    Current,
    /// The member's version is behind its home's publication.
    Behind,
    /// Its version cannot be compared without a successful publication read.
    Unchecked,
}
impl MemberBadge {
    /// Keep missing facts neutral rather than describing a failure.
    #[must_use]
    pub const fn variant(self) -> Variant {
        match self {
            Self::Current => Variant::Success,
            Self::Behind => Variant::Warning,
            Self::Unreported | Self::VersionUnreported | Self::Unchecked => Variant::Muted,
        }
    }
}
/// Complete row presentation; never four stacked sentences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRow {
    /// Nothing is repeated beneath a wholly unreported row.
    pub hint: Option<String>,
    /// One factual state, including a partial report without a version.
    pub badge: MemberBadge,
}
impl MemberRow {
    /// Preserve partial facts without treating a default empty count as zero.
    #[must_use]
    pub fn of(
        lang: Lang,
        member: &FleetMember,
        state: Option<MemberState>,
        now: DateTime<Utc>,
        offset: FixedOffset,
    ) -> Self {
        let badge = if member.applied.is_none() {
            if member.reported_at.is_some() || !member.conflicts.is_empty() {
                MemberBadge::VersionUnreported
            } else {
                MemberBadge::Unreported
            }
        } else {
            match state {
                Some(MemberState::UpToDate) => MemberBadge::Current,
                Some(MemberState::Behind(_)) => MemberBadge::Behind,
                Some(MemberState::NotYet) => MemberBadge::VersionUnreported,
                None => MemberBadge::Unchecked,
            }
        };
        Self {
            hint: lang.sync().member_hint(
                member.applied,
                state,
                member.reported_at.map(|at| Age::of(at, now, offset)),
            ),
            badge,
        }
    }
}
