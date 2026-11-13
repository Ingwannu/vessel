//! Due times that came while nothing ran them (ADR 0033): how many, which
//! one a catch-up run stands for, and whether one is made at all.
//!
//! The scheduler asks when it finds a schedule whose due time is at or
//! before the clock: normally a moment ago, after a sleeping machine or a
//! stopped node possibly many. One run at most comes of it, however many
//! there were (agent-orchestrator makes up to three, GOAT and Paseo one),
//! and none for a due time a day old: a prompt written for nine o'clock is
//! not sent at nine the next evening.

use chrono::{DateTime, TimeDelta, Utc};
use vessel_proto::automation::{CatchUp, RunTrigger, SkipReason};

use super::Due;

/// Due times counted, at most; a record says "1,000" beyond.
pub(crate) const COUNT_MAX: u32 = 1_000;

/// A due time noticed later than this is a missed one, not simply due.
pub(crate) fn late() -> TimeDelta {
    TimeDelta::seconds(60)
}

/// A missed due time older than this is not caught up.
pub(crate) fn too_old() -> TimeDelta {
    TimeDelta::hours(24)
}

/// The due times from a schedule's next due time up to the clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Missed {
    /// The first of them (the schedule's next due time).
    pub(crate) first: DateTime<Utc>,
    /// How many, up to [`COUNT_MAX`].
    pub(crate) count: u32,
    /// The latest of them, if it is less than [`too_old`] before the clock.
    pub(crate) recent: Option<DateTime<Utc>>,
    /// The first due time after the clock: the schedule's next.
    pub(crate) next: Option<DateTime<Utc>>,
}

/// What comes of [`Missed`] due times.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Decision {
    /// One run, for the due time `due_at`.
    Run {
        due_at: DateTime<Utc>,
        trigger: RunTrigger,
    },
    /// No run; a record of what was skipped, dated at the first.
    Skip {
        due_at: DateTime<Utc>,
        reason: SkipReason,
    },
}

/// The due times of `due` from `first` (the schedule's next due time, at
/// or before `now`) up to `now`.
pub(crate) fn missed(due: &Due<'_>, first: DateTime<Utc>, now: DateTime<Utc>) -> Missed {
    let window = now.checked_sub_signed(too_old()).unwrap_or(first);
    let mut count = 0_u32;
    let mut latest = None;
    let mut at = Some(first);
    while let Some(due_at) = at.filter(|due_at| *due_at <= now) {
        if count == COUNT_MAX {
            break;
        }
        count = count.saturating_add(1);
        latest = Some(due_at);
        at = due.next_after(due_at);
    }
    let walked_to_now = at.is_none_or(|due_at| due_at > now);
    let recent = if walked_to_now {
        latest.filter(|latest| *latest >= window)
    } else {
        // More than the count holds: find the latest from the start of the
        // window instead (at most a day of once-a-minute times).
        latest_in(due, first.max(window), now)
    };
    Missed {
        first,
        count,
        recent,
        next: due.next_after(now),
    }
}

/// The latest due time of `due` in `[from, to]`.
fn latest_in(due: &Due<'_>, from: DateTime<Utc>, to: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let mut at = from
        .checked_sub_signed(TimeDelta::nanoseconds(1))
        .and_then(|before| due.next_after(before));
    let mut latest = None;
    while let Some(due_at) = at.filter(|due_at| *due_at <= to) {
        latest = Some(due_at);
        at = due.next_after(due_at);
    }
    latest
}

impl Missed {
    /// One run or a record, by the schedule's `catch_up`: a due time
    /// noticed less than a minute late is simply due; otherwise, with
    /// `once`, the latest recent one is caught up; otherwise nothing runs.
    pub(crate) fn decide(&self, catch_up: CatchUp, now: DateTime<Utc>) -> Decision {
        if self.count == 1
            && let Some(recent) = self.recent
            && now.signed_duration_since(recent) < late()
        {
            return Decision::Run {
                due_at: recent,
                trigger: RunTrigger::Due,
            };
        }
        match (catch_up, self.recent) {
            (CatchUp::Once, Some(recent)) => Decision::Run {
                due_at: recent,
                trigger: RunTrigger::CaughtUp { missed: self.count },
            },
            (CatchUp::Once, None) | (CatchUp::Skip | CatchUp::Unknown, _) => Decision::Skip {
                due_at: self.first,
                reason: SkipReason::Missed { count: self.count },
            },
        }
    }
}
