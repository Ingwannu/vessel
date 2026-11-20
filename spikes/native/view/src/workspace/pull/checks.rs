//! Every check of a pull request as Paseo's checks section reads it
//! (`git/check-presentation.ts`, `pull-request-panel/checks-summary.ts`,
//! `checks-ring.tsx`, and the server's duration in `github-service.ts`
//! 4141-4175), from the runs a node at `PULL_REQUEST_PANE_PROTOCOL` sends
//! (ADR 0088 part 2).
//!
//! [Decision Log]
//! - Purpose: the pane's headline, its ring, its groups and each row's
//!   timing, decided once and tested outside the screen.
//! - Constraint: GitHub's checks carry no traits, so Paseo's
//!   `actionRequired`, `warning` and `manual` presentations never occur
//!   (they are GitLab's and Gitea's); an older node sends no runs, only
//!   counts and the failing ones.
//! - Choice: the four presentations GitHub reaches, in Paseo's order;
//!   [`ChecksOutcome`] for the headline; the ring as dash lengths in the
//!   ring's own units; a timing as seconds, worded by the language.
//! - Cost: a forge with traits needs the other three presentations back.

use chrono::{DateTime, Utc};
use vessel_proto::forge::{CheckRun, CheckState};

use super::facts::ChecksOutcome;

/// How a check is drawn and grouped (Paseo's `CheckPresentation`, the
/// values GitHub reaches).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Presentation {
    /// It failed.
    Failure,
    /// It runs or waits (and a state this app does not know).
    Pending,
    /// It passed.
    Success,
    /// Skipped or cancelled: says nothing about the run.
    Ignored,
}

impl Presentation {
    /// Paseo's `PRESENTATION_ORDER`: what to act on, then what runs, then
    /// what needs no attention, skipped last.
    pub const ORDER: [Self; 4] = [Self::Failure, Self::Pending, Self::Success, Self::Ignored];

    /// Paseo's `classifyCheck` for a check with no traits.
    #[must_use]
    pub const fn of(state: CheckState) -> Self {
        match state {
            CheckState::Passed => Self::Success,
            CheckState::Failed => Self::Failure,
            CheckState::Cancelled | CheckState::Skipped => Self::Ignored,
            CheckState::Pending | CheckState::Unknown => Self::Pending,
        }
    }
}

/// The checks of one presentation, in the order they came.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group<'a> {
    /// Their presentation.
    pub presentation: Presentation,
    /// The checks.
    pub checks: Vec<&'a CheckRun>,
}

/// Everything the checks section draws (Paseo's `ChecksSummary`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary<'a> {
    /// The headline's outcome.
    pub outcome: ChecksOutcome,
    /// The non-empty groups, in [`Presentation::ORDER`].
    pub groups: Vec<Group<'a>>,
    /// Every check, skipped ones included.
    pub total: usize,
}

impl Summary<'_> {
    /// Each group's presentation and count, for the detail line and the
    /// ring.
    #[must_use]
    pub fn parts(&self) -> Vec<(Presentation, usize)> {
        self.groups
            .iter()
            .map(|group| (group.presentation, group.checks.len()))
            .collect()
    }
}

/// `runs` summarized: a failure outranks anything still running.
#[must_use]
pub fn summary(runs: &[CheckRun]) -> Summary<'_> {
    let groups: Vec<Group<'_>> = Presentation::ORDER
        .into_iter()
        .filter_map(|presentation| {
            let checks: Vec<&CheckRun> = runs
                .iter()
                .filter(|run| Presentation::of(run.state) == presentation)
                .collect();
            (!checks.is_empty()).then_some(Group {
                presentation,
                checks,
            })
        })
        .collect();
    let has = |presentation| groups.iter().any(|group| group.presentation == presentation);
    let outcome = if runs.is_empty() {
        ChecksOutcome::None
    } else if has(Presentation::Failure) {
        ChecksOutcome::Failure
    } else if has(Presentation::Pending) {
        ChecksOutcome::Pending
    } else {
        ChecksOutcome::Success
    };
    Summary {
        outcome,
        groups,
        total: runs.len(),
    }
}

/// The ring's box (Paseo's `ICON_SIZE.lg`).
pub const RING_SIZE: f32 = 20.0;
/// The ring's stroke.
pub const RING_STROKE: f32 = 3.0;
/// The ring's radius: half the box less half the stroke, (20 - 3) / 2.
pub const RING_RADIUS: f32 = 8.5;
/// The gap between two segments, along the ring.
const SEGMENT_GAP: f32 = 2.0;
/// The shortest segment drawn.
const MIN_SEGMENT: f32 = 1.5;

/// One coloured stretch of the ring, from twelve o'clock clockwise, in
/// the ring's own length units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    /// Its colour's presentation.
    pub presentation: Presentation,
    /// Where it starts along the ring.
    pub start: f32,
    /// Its length.
    pub length: f32,
}

/// The ring's circumference.
#[must_use]
#[expect(
    clippy::float_arithmetic,
    reason = "a ring's length is geometry for drawing, never a count"
)]
pub fn circumference() -> f32 {
    std::f32::consts::TAU * RING_RADIUS
}

/// Paseo's ring: one segment per part, each its share of the ring less a
/// gap (never shorter than the minimum); one part fills the ring.
#[must_use]
#[expect(
    clippy::float_arithmetic,
    reason = "segment lengths are geometry for drawing, never counts"
)]
pub fn segments(parts: &[(Presentation, usize)], total: usize) -> Vec<Segment> {
    if total == 0 {
        return Vec::new();
    }
    let whole = circumference();
    let single = parts.len() == 1;
    let mut start = 0.0;
    parts
        .iter()
        .map(|&(presentation, count)| {
            let arc = share(count, total) * whole;
            let length = if single {
                whole
            } else {
                (arc - SEGMENT_GAP).max(MIN_SEGMENT)
            };
            let segment = Segment {
                presentation,
                start,
                length,
            };
            start += arc;
            segment
        })
        .collect()
}

/// `count` of `total` as a fraction (both at most a few hundred).
#[expect(
    clippy::float_arithmetic,
    reason = "a share of the ring is geometry for drawing, never a count"
)]
fn share(count: usize, total: usize) -> f32 {
    let count = u16::try_from(count).unwrap_or(u16::MAX);
    let total = u16::try_from(total).unwrap_or(u16::MAX).max(1);
    f32::from(count) / f32::from(total)
}

/// How long a check ran, as its row says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// It still runs ("running 2m").
    pub running: bool,
    /// Whole seconds from its start to its end, or to now while it runs.
    pub seconds: u64,
}

/// A check's timing: none without a start, or with an end before it.
#[must_use]
pub fn timing(run: &CheckRun, now: DateTime<Utc>) -> Option<Timing> {
    let started = run.started_at?;
    let ended = run.completed_at.unwrap_or(now);
    let seconds = u64::try_from(ended.signed_duration_since(started).num_seconds()).ok()?;
    Some(Timing {
        running: Presentation::of(run.state) == Presentation::Pending,
        seconds,
    })
}

/// Paseo's server's duration: hours, minutes and seconds, zero hours and
/// minutes left out, seconds kept when nothing else is ("1h 5m",
/// "1m 30s", "45s", "0s").
#[must_use]
pub fn duration(seconds: u64) -> String {
    let hours = seconds.checked_div(3600).unwrap_or_default();
    let minutes = seconds
        .checked_rem(3600)
        .and_then(|within| within.checked_div(60))
        .unwrap_or_default();
    let rest = seconds.checked_rem(60).unwrap_or_default();
    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if rest > 0 || parts.is_empty() {
        parts.push(format!("{rest}s"));
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests;
