//! The checks section's reading of every check, against Paseo's cases.

use chrono::{Duration, TimeZone as _, Utc};
use vessel_proto::forge::{CheckRun, CheckState};

use super::*;

fn run(name: &str, state: CheckState) -> CheckRun {
    CheckRun::named(name.parse().unwrap(), state, None)
}

#[test]
fn checks_group_in_paseo_s_order_and_keep_their_own_order_inside() {
    let runs = vec![
        run("lint", CheckState::Passed),
        run("docs", CheckState::Skipped),
        run("build", CheckState::Failed),
        run("test", CheckState::Pending),
        run("deploy", CheckState::Cancelled),
        run("e2e", CheckState::Passed),
        run("new", CheckState::Unknown),
    ];
    let read = summary(&runs);
    assert_eq!(
        read.outcome,
        ChecksOutcome::Failure,
        "a failure outranks a run"
    );
    assert_eq!(read.total, 7, "skipped ones count in the total");
    assert_eq!(
        read.parts(),
        [
            (Presentation::Failure, 1),
            (Presentation::Pending, 2),
            (Presentation::Success, 2),
            (Presentation::Ignored, 2),
        ]
    );
    let passed: Vec<&str> = read.groups[2]
        .checks
        .iter()
        .map(|check| check.name.as_str())
        .collect();
    assert_eq!(passed, ["lint", "e2e"]);
}

#[test]
fn the_outcome_is_the_worst_thing_happening() {
    assert_eq!(summary(&[]).outcome, ChecksOutcome::None);
    let running = [run("a", CheckState::Passed), run("b", CheckState::Pending)];
    assert_eq!(summary(&running).outcome, ChecksOutcome::Pending);
    let done = [run("a", CheckState::Passed), run("b", CheckState::Skipped)];
    assert_eq!(summary(&done).outcome, ChecksOutcome::Success);
    let skipped = [run("a", CheckState::Skipped)];
    assert_eq!(
        summary(&skipped).outcome,
        ChecksOutcome::Success,
        "all skipped reads as passed, as Paseo's"
    );
}

#[test]
fn the_ring_is_one_segment_per_part_with_gaps() {
    let whole = circumference();
    assert!((whole - 2.0 * std::f32::consts::PI * 8.5).abs() < 1e-4);
    assert!(segments(&[], 0).is_empty());
    let one = segments(&[(Presentation::Success, 3)], 3);
    assert_eq!(one.len(), 1);
    assert!(
        (one[0].length - whole).abs() < 1e-4,
        "one part fills the ring"
    );
    let two = segments(&[(Presentation::Failure, 1), (Presentation::Success, 3)], 4);
    let quarter = whole / 4.0;
    assert!((two[0].start).abs() < 1e-4);
    assert!((two[0].length - (quarter - 2.0)).abs() < 1e-4);
    assert!(
        (two[1].start - quarter).abs() < 1e-4,
        "starts after the whole share"
    );
    assert!((two[1].length - (3.0 * quarter - 2.0)).abs() < 1e-4);
    let tiny = segments(
        &[(Presentation::Failure, 1), (Presentation::Success, 99)],
        100,
    );
    assert!(
        (tiny[0].length - 1.5).abs() < 1e-4,
        "never shorter than the minimum"
    );
}

#[test]
fn a_timing_runs_from_the_start_to_the_end_or_now() {
    let start = Utc.with_ymd_and_hms(2026, 11, 19, 10, 0, 0).unwrap();
    let now = start + Duration::seconds(150);
    let mut done = run("build", CheckState::Passed);
    assert_eq!(timing(&done, now), None, "no start, no timing");
    done.started_at = Some(start);
    done.completed_at = Some(start + Duration::seconds(90));
    assert_eq!(
        timing(&done, now),
        Some(Timing {
            running: false,
            seconds: 90
        })
    );
    let mut running = run("test", CheckState::Pending);
    running.started_at = Some(start);
    assert_eq!(
        timing(&running, now),
        Some(Timing {
            running: true,
            seconds: 150
        })
    );
    let mut backwards = done.clone();
    backwards.completed_at = Some(start - Duration::seconds(1));
    assert_eq!(timing(&backwards, now), None, "an end before its start");
}

#[test]
fn a_duration_reads_as_paseo_s_server_writes_it() {
    assert_eq!(duration(0), "0s");
    assert_eq!(duration(45), "45s");
    assert_eq!(duration(90), "1m 30s");
    assert_eq!(duration(120), "2m");
    assert_eq!(duration(3900), "1h 5m");
    assert_eq!(duration(3601), "1h 1s");
}
