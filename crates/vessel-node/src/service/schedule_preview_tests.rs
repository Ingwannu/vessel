//! Preview boundaries, reference cases, and cancellation accounting.
use super::*;
use vessel_proto::automation::{EverySeconds, PreviewCount};

fn at(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}
fn request(cadence: Cadence, after: &str, count: u32) -> PreviewSchedule {
    PreviewSchedule {
        cadence,
        after: Some(at(after)),
        count: PreviewCount::try_from(count).unwrap(),
    }
}
fn cron(expr: &str, zone: &str) -> Cadence {
    Cadence::Cron {
        expr: expr.parse().unwrap(),
        zone: zone.parse().unwrap(),
    }
}
fn preview(request: PreviewSchedule) -> Result<SchedulePreview, SchedulePreviewError> {
    compute(request, at("2026-01-01T00:00:00Z"))
}

#[test]
fn paseos_utc_day_and_zone_cases_use_the_schedulers_exact_results() {
    // Paseo server/schedule/cron.test.ts, including its restricted-day cases.
    for &(expr, zone, after, next) in vessel_testkit::schedule_fixtures::CRON_CASES {
        let result = preview(request(cron(expr, zone), after, 1)).unwrap();
        assert_eq!(result.runs, [at(next)], "{expr} {zone} {after}");
    }
}

#[test]
fn gaps_run_at_jump_and_folds_once_as_adr_0033_decides() {
    let gap = preview(request(
        cron("30 2 * * *", "America/New_York"),
        "2026-03-08T06:59:00Z",
        2,
    ))
    .unwrap();
    assert_eq!(
        gap.runs,
        [at("2026-03-08T07:00:00Z"), at("2026-03-09T06:30:00Z")]
    );
    let fold = preview(request(
        cron("30 1 * * *", "America/New_York"),
        "2026-11-01T05:29:30Z",
        2,
    ))
    .unwrap();
    assert_eq!(
        fold.runs,
        [at("2026-11-01T05:30:00Z"), at("2026-11-02T06:30:00Z")]
    );
}

#[test]
fn an_effective_anchor_and_fractional_boundaries_survive_saving() {
    let interval = |anchor| Cadence::Every {
        seconds: EverySeconds::try_from(300).unwrap(),
        anchor,
    };
    let result = preview(request(interval(None), "2026-01-01T00:00:00.123456789Z", 2)).unwrap();
    assert_eq!(result.cadence, interval(Some(result.after)));
    assert_eq!(
        result.runs,
        [
            at("2026-01-01T00:05:00.123456789Z"),
            at("2026-01-01T00:10:00.123456789Z")
        ]
    );
    for (after, expected) in [
        ("2025-12-31T23:59:59Z", "2026-01-01T00:00:00Z"),
        ("2026-01-01T00:05:00Z", "2026-01-01T00:10:00Z"),
        ("2026-01-01T00:04:59.999999999Z", "2026-01-01T00:05:00Z"),
    ] {
        let result = preview(request(
            interval(Some(at("2026-01-01T00:00:00Z"))),
            after,
            1,
        ))
        .unwrap();
        assert_eq!(result.runs, [at(expected)]);
    }
    let request = PreviewSchedule {
        cadence: interval(None),
        after: None,
        count: PreviewCount::default(),
    };
    assert_eq!(preview(request).unwrap().after, at("2026-01-01T00:00:00Z"));
}

#[test]
fn errors_are_typed_and_never_return_partial_results() {
    for (cadence, after, count, error) in [
        (
            cron("0 9 * * *", "Not/AZone"),
            "2026-01-01T00:00:00Z",
            1,
            SchedulePreviewError::UnknownZone,
        ),
        (
            Cadence::Unknown,
            "2026-01-01T00:00:00Z",
            1,
            SchedulePreviewError::UnknownCadence,
        ),
        (
            cron("0 0 30 2 *", "UTC"),
            "2026-01-01T00:00:00Z",
            1,
            SchedulePreviewError::NoFutureRun,
        ),
        (
            cron("* * * * *", "UTC"),
            "2016-12-31T23:59:60Z",
            1,
            SchedulePreviewError::TimeOutOfRange,
        ),
        (
            cron("* * * * *", "UTC"),
            "+010000-01-01T00:00:00Z",
            1,
            SchedulePreviewError::TimeOutOfRange,
        ),
        (
            cron("* * * * *", "UTC"),
            "9999-12-31T23:55:00Z",
            10,
            SchedulePreviewError::TimeOutOfRange,
        ),
    ] {
        assert_eq!(preview(request(cadence, after, count)), Err(error));
        let wire = refusal(error);
        assert_eq!(wire.code, ErrorCode::InvalidRequest);
        assert_eq!(
            wire.detail,
            Some(ErrorDetail::SchedulePreview { reason: error })
        );
    }
}

#[test]
fn running_out_of_range_after_some_runs_never_returns_a_partial_list() {
    // Jiff reserves enough room for every legal zone offset. Its actual
    // timestamp bound, rather than the last day of year 9999, is the edge.
    let after = DateTime::from_timestamp(jiff::Timestamp::MAX.as_second(), 0)
        .unwrap()
        .checked_sub_signed(chrono::TimeDelta::minutes(5))
        .unwrap();
    let request = |cadence| PreviewSchedule {
        cadence,
        after: Some(after),
        count: PreviewCount::try_from(10).unwrap(),
    };
    assert_eq!(
        preview(request(cron("* * * * *", "UTC"))),
        Err(SchedulePreviewError::NoFutureRun)
    );
    let every = Cadence::Every {
        seconds: EverySeconds::try_from(60).unwrap(),
        anchor: None,
    };
    assert_eq!(
        preview(request(every)),
        Err(SchedulePreviewError::TimeOutOfRange)
    );
}

#[tokio::test]
async fn cancellation_keeps_the_slot_until_blocking_work_finishes() {
    let slots = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let (entered, started) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::sync_channel(1);
    let (done, finished) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(execute(std::sync::Arc::clone(&slots), move || {
        entered.send(()).unwrap();
        wait.recv().unwrap();
        done.send(()).unwrap();
        Ok(())
    }));
    started.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let busy = execute(std::sync::Arc::clone(&slots), || Ok(()))
        .await
        .unwrap_err();
    assert_eq!(busy.code, ErrorCode::Unavailable);
    assert_eq!(slots.available_permits(), 0);
    release.send(()).unwrap();
    finished.await.unwrap();
    let permit = slots.acquire().await.unwrap();
    drop(permit);
    execute(slots, || Ok(())).await.unwrap();
}
