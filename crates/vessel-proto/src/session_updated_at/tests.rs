//! Ordering remains strict across backwards wall time and persistence bounds.

use super::{InvalidSessionUpdatedAt, SessionUpdatedAt};
use chrono::{DateTime, TimeDelta, Utc};

fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

#[test]
fn mutation_clock_round_trips_whole_milliseconds_and_rejects_fractional_values() {
    let time = at("2026-01-01T00:00:00.123Z");
    let value = SessionUpdatedAt::try_from(time).unwrap();
    let encoded = serde_json::to_string(&value).unwrap();
    assert_eq!(
        serde_json::from_str::<SessionUpdatedAt>(&encoded).unwrap(),
        value
    );
    assert_eq!(value.time(), time);
    let fractional = at("2026-01-01T00:00:00.123001Z");
    assert_eq!(
        SessionUpdatedAt::try_from(fractional),
        Err(InvalidSessionUpdatedAt::Submillisecond)
    );
    assert!(
        serde_json::from_str::<SessionUpdatedAt>(&serde_json::to_string(&fractional).unwrap())
            .is_err()
    );
    assert_eq!(
        SessionUpdatedAt::try_from(DateTime::<Utc>::MAX_UTC),
        Err(InvalidSessionUpdatedAt::OutOfRange)
    );
}

#[test]
fn unknown_initializes_and_repeated_or_backwards_wall_time_advances_strictly() {
    let now = at("2026-01-01T00:00:00.123456Z");
    let first = SessionUpdatedAt::advance(None, now).unwrap();
    assert_eq!(first.time(), at("2026-01-01T00:00:00.123Z"));
    let second = SessionUpdatedAt::advance(Some(first), now).unwrap();
    assert_eq!(
        second.time(),
        first
            .time()
            .checked_add_signed(SessionUpdatedAt::MIN_INCREMENT)
            .unwrap()
    );
    let old = now.checked_sub_signed(TimeDelta::days(1)).unwrap();
    let third = SessionUpdatedAt::advance(Some(second), old).unwrap();
    assert!(third > second);
    assert_eq!(
        third.time(),
        second
            .time()
            .checked_add_signed(SessionUpdatedAt::MIN_INCREMENT)
            .unwrap()
    );
    let later = now.checked_add_signed(TimeDelta::days(1)).unwrap();
    assert_eq!(
        SessionUpdatedAt::advance(Some(third), later)
            .unwrap()
            .time(),
        DateTime::from_timestamp_millis(later.timestamp_millis()).unwrap()
    );
}

#[test]
fn exhausted_stored_time_refuses_instead_of_saturating() {
    let limit = DateTime::<Utc>::from_timestamp_nanos(i64::MAX);
    let rounded = DateTime::from_timestamp_millis(limit.timestamp_millis()).unwrap();
    let last = SessionUpdatedAt::try_from(rounded).unwrap();
    assert_eq!(
        SessionUpdatedAt::advance(Some(last), rounded),
        Err(InvalidSessionUpdatedAt::Exhausted)
    );
    assert_eq!(
        SessionUpdatedAt::advance(None, DateTime::<Utc>::MAX_UTC),
        Err(InvalidSessionUpdatedAt::OutOfRange)
    );
}
