//! Due times on a clock the tests give: zones, daylight-saving gaps and
//! folds, elapsed intervals, and missed due times. Nothing waits.

use chrono::{DateTime, TimeDelta, TimeZone as _, Utc};
use vessel_proto::automation::{Cadence, CatchUp, EverySeconds, RunTrigger, SkipReason};

use super::{Decision, Due, NotComputable, Zones, missed};

fn utc(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

fn cron(expr: &str, zone: &str) -> Cadence {
    Cadence::Cron {
        expr: expr.parse().unwrap(),
        zone: zone.parse().unwrap(),
    }
}

fn every(seconds: u32, anchor: Option<&str>) -> Cadence {
    Cadence::Every {
        seconds: EverySeconds::try_from(seconds).unwrap(),
        anchor: anchor.map(utc),
    }
}

/// The due times of `cadence` after `from`, `count` of them.
fn times(cadence: &Cadence, from: &str, count: usize) -> Vec<DateTime<Utc>> {
    let zones = Zones::default();
    let due = Due::of(cadence, utc(from), &zones).unwrap();
    let mut at = utc(from);
    let mut found = Vec::new();
    for _ in 0..count {
        at = due.next_after(at).unwrap();
        found.push(at);
    }
    found
}

#[test]
fn weekday_mornings_in_a_zone() {
    // Seoul is UTC+9 with no daylight saving: 09:00 there is 00:00 UTC.
    let found = times(
        &cron("0 9 * * 1-5", "Asia/Seoul"),
        "2026-11-13T01:00:00Z",
        3,
    );
    // Friday the 13th has passed 09:00 in Seoul; Monday, Tuesday follow.
    assert_eq!(
        found,
        [
            utc("2026-11-16T00:00:00Z"),
            utc("2026-11-17T00:00:00Z"),
            utc("2026-11-18T00:00:00Z"),
        ]
    );
}

#[test]
fn a_due_time_is_strictly_after_the_instant_asked_about() {
    let cadence = cron("*/30 * * * *", "UTC");
    assert_eq!(
        times(&cadence, "2026-01-01T10:00:00Z", 1),
        [utc("2026-01-01T10:30:00Z")]
    );
    assert_eq!(
        times(&cadence, "2026-01-01T09:59:59.5Z", 1),
        [utc("2026-01-01T10:00:00Z")]
    );
}

#[test]
fn a_time_the_clocks_skip_runs_once_when_they_jump() {
    // New York, 8 March 2026: 02:00 EST becomes 03:00 EDT (07:00 UTC).
    let half_past_two = cron("30 2 * * *", "America/New_York");
    assert_eq!(
        times(&half_past_two, "2026-03-06T12:00:00Z", 3),
        [
            // 02:30 EST on the 7th.
            utc("2026-03-07T07:30:00Z"),
            // No 02:30 on the 8th: the moment the clocks jump.
            utc("2026-03-08T07:00:00Z"),
            // 02:30 EDT on the 9th.
            utc("2026-03-09T06:30:00Z"),
        ]
    );
    // Four times inside the gap make one run.
    let quarter_hours = cron("*/15 2 * * *", "America/New_York");
    assert_eq!(
        times(&quarter_hours, "2026-03-08T06:00:00Z", 2),
        [utc("2026-03-08T07:00:00Z"), utc("2026-03-09T06:00:00Z")]
    );
    // A time in the gap and 03:00 itself are one instant, so one run.
    let both = cron("30 2,3 * * *", "America/New_York");
    assert_eq!(
        times(&both, "2026-03-08T06:00:00Z", 2),
        [utc("2026-03-08T07:00:00Z"), utc("2026-03-08T07:30:00Z")]
    );
}

#[test]
fn a_time_the_clocks_repeat_runs_once_the_first_time() {
    // New York, 1 November 2026: 02:00 EDT becomes 01:00 EST (06:00 UTC);
    // 01:30 happens at 05:30 and again at 06:30 UTC.
    let half_past_one = cron("30 1 * * *", "America/New_York");
    assert_eq!(
        times(&half_past_one, "2026-11-01T04:00:00Z", 2),
        [utc("2026-11-01T05:30:00Z"), utc("2026-11-02T06:30:00Z")]
    );
    // Asked from inside the second pass, the repeated times are not due
    // again: the next is the next day's.
    assert_eq!(
        times(&half_past_one, "2026-11-01T06:10:00Z", 1),
        [utc("2026-11-02T06:30:00Z")]
    );
    // Every minute of the repeated hour runs once.
    let each_minute = cron("* 1 * * *", "America/New_York");
    let found = times(&each_minute, "2026-11-01T04:59:30Z", 61);
    assert_eq!(found.first(), Some(&utc("2026-11-01T05:00:00Z")));
    assert_eq!(found.get(59), Some(&utc("2026-11-01T05:59:00Z")));
    assert_eq!(found.get(60), Some(&utc("2026-11-02T06:00:00Z")));
}

#[test]
fn half_hour_shifts_and_other_hemispheres() {
    // Lord Howe Island moves its clocks by thirty minutes: on 4 October
    // 2026 02:00 becomes 02:30 (+10:30 to +11, 15:30 UTC the 3rd).
    let quarter_past_two = cron("15 2 * * *", "Australia/Lord_Howe");
    assert_eq!(
        times(&quarter_past_two, "2026-10-03T12:00:00Z", 2),
        [utc("2026-10-03T15:30:00Z"), utc("2026-10-04T15:15:00Z")]
    );
    // London, 29 March 2026: 01:00 GMT becomes 02:00 BST.
    let half_past_one = cron("30 1 * * *", "Europe/London");
    assert_eq!(
        times(&half_past_one, "2026-03-28T12:00:00Z", 2),
        [utc("2026-03-29T01:00:00Z"), utc("2026-03-30T00:30:00Z")]
    );
}

#[test]
fn every_counts_elapsed_time_from_its_anchor() {
    // No anchor: the cadence was set at `from`, so the first is an
    // interval later, never at once.
    assert_eq!(
        times(&every(300, None), "2026-01-01T10:00:00Z", 2),
        [utc("2026-01-01T10:05:00Z"), utc("2026-01-01T10:10:00Z")]
    );
    // An anchor in the future is the first due time.
    assert_eq!(
        times(
            &every(3600, Some("2026-01-02T09:00:00Z")),
            "2026-01-01T10:00:00Z",
            2
        ),
        [utc("2026-01-02T09:00:00Z"), utc("2026-01-02T10:00:00Z")]
    );
    // An anchor in the past keeps its grid.
    assert_eq!(
        times(
            &every(3600, Some("2026-01-01T08:20:00Z")),
            "2026-01-01T10:00:00Z",
            1
        ),
        [utc("2026-01-01T10:20:00Z")]
    );
    // A day is 86,400 seconds across a daylight-saving change.
    let daily = every(86_400, Some("2026-03-07T14:00:00Z"));
    assert_eq!(
        times(&daily, "2026-03-07T14:00:00Z", 2),
        [utc("2026-03-08T14:00:00Z"), utc("2026-03-09T14:00:00Z")]
    );
    // Fractions of a second in the anchor do not make a time equal to
    // the one asked about.
    let anchor = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + TimeDelta::milliseconds(500);
    let zones = Zones::default();
    let cadence = every(60, None);
    let due = Due::of(&cadence, anchor, &zones).unwrap();
    let first = due.next_after(anchor).unwrap();
    assert_eq!(first, anchor + TimeDelta::seconds(60));
    assert_eq!(
        due.next_after(first).unwrap(),
        anchor + TimeDelta::seconds(120)
    );
}

#[test]
fn a_cron_that_matches_no_day_has_no_due_time() {
    let zones = Zones::default();
    let never = cron("0 0 30 2 *", "UTC");
    let due = Due::of(&never, utc("2026-01-01T00:00:00Z"), &zones).unwrap();
    assert_eq!(due.next_after(utc("2026-01-01T00:00:00Z")), None);
    // 29 February is found, eight years on across 2100.
    let leap = cron("0 12 29 2 *", "UTC");
    assert_eq!(
        times(&leap, "2026-01-01T00:00:00Z", 1),
        [utc("2028-02-29T12:00:00Z")]
    );
    assert_eq!(
        times(&leap, "2096-03-01T00:00:00Z", 1),
        [utc("2104-02-29T12:00:00Z")]
    );
}

#[test]
fn an_unknown_zone_is_refused() {
    let zones = Zones::default();
    let cadence = cron("0 9 * * *", "Mars/Olympus_Mons");
    assert!(matches!(
        Due::of(&cadence, utc("2026-01-01T00:00:00Z"), &zones),
        Err(NotComputable::Zone(_))
    ));
    assert!(matches!(
        Due::of(&Cadence::Unknown, utc("2026-01-01T00:00:00Z"), &zones),
        Err(NotComputable::Unknown)
    ));
}

fn decide(
    cadence: &Cadence,
    first: &str,
    now: &str,
    catch_up: CatchUp,
) -> (Decision, Option<DateTime<Utc>>) {
    let zones = Zones::default();
    let due = Due::of(cadence, utc("2026-01-01T00:00:00Z"), &zones).unwrap();
    let missed = missed(&due, utc(first), utc(now));
    (missed.decide(catch_up, utc(now)), missed.next)
}

#[test]
fn a_due_time_noticed_at_once_is_simply_due() {
    let hourly = cron("0 * * * *", "UTC");
    let (decision, next) = decide(
        &hourly,
        "2026-01-01T10:00:00Z",
        "2026-01-01T10:00:00.003Z",
        CatchUp::Skip,
    );
    assert_eq!(
        decision,
        Decision::Run {
            due_at: utc("2026-01-01T10:00:00Z"),
            trigger: RunTrigger::Due
        }
    );
    assert_eq!(next, Some(utc("2026-01-01T11:00:00Z")));
}

#[test]
fn many_missed_times_make_one_run_for_the_latest() {
    // Every five minutes; the machine slept from 10:00 to 13:02.
    let five = every(300, Some("2026-01-01T00:00:00Z"));
    let (decision, next) = decide(
        &five,
        "2026-01-01T10:00:00Z",
        "2026-01-01T13:02:00Z",
        CatchUp::Once,
    );
    assert_eq!(
        decision,
        Decision::Run {
            due_at: utc("2026-01-01T13:00:00Z"),
            trigger: RunTrigger::CaughtUp { missed: 37 }
        }
    );
    assert_eq!(next, Some(utc("2026-01-01T13:05:00Z")));
    // One daily time missed by hours is still caught up.
    let nine = cron("0 9 * * *", "UTC");
    let (decision, _) = decide(
        &nine,
        "2026-01-01T09:00:00Z",
        "2026-01-01T15:00:00Z",
        CatchUp::Once,
    );
    assert_eq!(
        decision,
        Decision::Run {
            due_at: utc("2026-01-01T09:00:00Z"),
            trigger: RunTrigger::CaughtUp { missed: 1 }
        }
    );
}

#[test]
fn missed_times_are_skipped_when_asked_or_a_day_old() {
    let nine = cron("0 9 * * *", "UTC");
    let (skipped, next) = decide(
        &nine,
        "2026-01-01T09:00:00Z",
        "2026-01-01T15:00:00Z",
        CatchUp::Skip,
    );
    assert_eq!(
        skipped,
        Decision::Skip {
            due_at: utc("2026-01-01T09:00:00Z"),
            reason: SkipReason::Missed { count: 1 }
        }
    );
    assert_eq!(next, Some(utc("2026-01-02T09:00:00Z")));
    // A node stopped for three weeks: 21 missed, the latest 6 hours old,
    // one run.
    let (three_weeks, _) = decide(
        &nine,
        "2026-01-01T09:00:00Z",
        "2026-01-21T15:00:00Z",
        CatchUp::Once,
    );
    assert_eq!(
        three_weeks,
        Decision::Run {
            due_at: utc("2026-01-21T09:00:00Z"),
            trigger: RunTrigger::CaughtUp { missed: 21 }
        }
    );
    // A weekly time last due two days ago is not caught up.
    let weekly = cron("0 9 * * 1", "UTC");
    let (old, _) = decide(
        &weekly,
        "2026-01-05T09:00:00Z",
        "2026-01-07T09:00:00Z",
        CatchUp::Once,
    );
    assert_eq!(
        old,
        Decision::Skip {
            due_at: utc("2026-01-05T09:00:00Z"),
            reason: SkipReason::Missed { count: 1 }
        }
    );
}

#[test]
fn a_count_past_its_bound_still_finds_the_latest() {
    // Every minute, the node stopped for three days: more than 1,000
    // missed; the run is for the minute just past.
    let each = cron("* * * * *", "UTC");
    let (decision, next) = decide(
        &each,
        "2026-01-01T00:00:00Z",
        "2026-01-04T00:00:30Z",
        CatchUp::Once,
    );
    assert_eq!(
        decision,
        Decision::Run {
            due_at: utc("2026-01-04T00:00:00Z"),
            trigger: RunTrigger::CaughtUp { missed: 1_000 }
        }
    );
    assert_eq!(next, Some(utc("2026-01-04T00:01:00Z")));
    // Every minute in January only, stopped until March: nothing recent.
    let january = cron("* * * 1 *", "UTC");
    let (decision, next) = decide(
        &january,
        "2026-01-01T00:00:00Z",
        "2026-03-01T00:00:00Z",
        CatchUp::Once,
    );
    assert_eq!(
        decision,
        Decision::Skip {
            due_at: utc("2026-01-01T00:00:00Z"),
            reason: SkipReason::Missed { count: 1_000 }
        }
    );
    assert_eq!(next, Some(utc("2027-01-01T00:00:00Z")));
}
