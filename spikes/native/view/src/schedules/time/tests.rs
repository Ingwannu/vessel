//! Same English inputs/outputs as utils/time.test.ts and schedule-format.test.ts.
use super::super::tests::visible;
use super::*;
use crate::localization::{ENGLISH, Locale};
fn date(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}
#[derive(serde::Deserialize)]
struct Case {
    id: String,
    at: DateTime<Utc>,
    en: String,
    ko: String,
    en_fluent: String,
    ko_fluent: String,
}
#[test]
fn reference_past_ages_and_exact_week_boundary_in_both_languages() {
    let now = date("2026-07-16T12:00:00Z");
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    // Paseo utils/time.test.ts::formatTimeAgo and describeTimeAgo.
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../../../tests/fixtures/schedules/ages.json")).unwrap();
    for case in cases {
        let age = Age::of(case.at, now);
        let english = super::super::metadata::age(age, |_| None, en).unwrap();
        let korean = super::super::metadata::age(age, |_| None, ko).unwrap();
        assert_eq!(english, case.en_fluent, "{}", case.id);
        assert_eq!(korean, case.ko_fluent, "{}", case.id);
        assert_eq!(visible(&english), case.en, "{}", case.id);
        assert_eq!(visible(&korean), case.ko, "{}", case.id);
    }
    let exact = now - chrono::TimeDelta::days(7);
    assert_eq!(Age::of(exact, now), Age::Date(exact));
    assert_eq!(
        Age::of(exact + chrono::TimeDelta::milliseconds(1), now),
        Age::Days(6)
    );
}
#[test]
fn reference_future_distances_and_half_up_edges() {
    let now = date("2026-01-01T00:00:00Z");
    let en = Translator::get(ENGLISH).unwrap();
    // Paseo schedule-format.test.ts::formatNextRun.
    for (at, expected) in [
        ("2026-01-01T00:00:15Z", "soon"),
        ("2026-01-01T00:30:00Z", "in 30m"),
        ("2026-01-01T03:00:00Z", "in 3h"),
        ("2026-01-03T00:00:00Z", "in 2d"),
        ("2025-12-31T23:59:00Z", "soon"),
        ("2026-01-01T00:01:30Z", "in 2m"),
        ("2026-01-01T00:59:59Z", "in 60m"),
        ("2026-01-01T01:30:00Z", "in 2h"),
    ] {
        assert_eq!(
            visible(&Countdown::of(date(at), now).label(en).unwrap()),
            expected
        );
    }
    for at in [DateTime::<Utc>::MIN_UTC, DateTime::<Utc>::MAX_UTC] {
        let _total = Countdown::of(at, now);
        let _total = Age::of(at, now);
    }
}

#[test]
fn fluent_directional_boundaries_are_retained_in_production_distance_text() {
    let en = Translator::get(ENGLISH).unwrap();
    assert_eq!(
        Countdown::Minutes(30).label(en).unwrap(),
        "in \u{2068}30\u{2069}m"
    );
}

#[test]
fn reference_tiers_and_wall_clock_alignment_need_no_static_timer() {
    assert_eq!(Age::Now.refresh_tier(), Some(RefreshTier::Minute));
    assert_eq!(Age::Minutes(5).refresh_tier(), Some(RefreshTier::Minute));
    assert_eq!(Age::Hours(3).refresh_tier(), Some(RefreshTier::HalfHour));
    assert_eq!(Age::Days(3).refresh_tier(), Some(RefreshTier::Hour));
    assert_eq!(Age::Date(DateTime::UNIX_EPOCH).refresh_tier(), None);
    for (at, minute, half_hour, hour) in [
        ("2026-01-01T00:59:59Z", 1, 1, 1),
        ("2026-01-01T00:00:00Z", 60, 1800, 3600),
        ("1969-12-31T23:59:59Z", 1, 1, 1),
    ] {
        for (tier, seconds) in [
            (RefreshTier::Minute, minute),
            (RefreshTier::HalfHour, half_hour),
            (RefreshTier::Hour, hour),
        ] {
            assert_eq!(
                tier.after(date(at)),
                std::time::Duration::from_secs(seconds)
            );
        }
    }
}
