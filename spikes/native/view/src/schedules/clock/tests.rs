//! Civil conversion edges, exact language errors and package pin consistency.
use super::super::tests::visible;
use super::*;
use crate::localization::{ENGLISH, Locale};
fn date(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}
#[test]
fn viewer_calendar_uses_the_instant_across_gap_fold_and_midnight() {
    let clock = Clock::known("America/New_York".parse().unwrap()).unwrap();
    for (at, local) in [
        ("2025-03-09T06:30:00Z", "2025-03-09T01:30:00"),
        ("2025-03-09T07:30:00Z", "2025-03-09T03:30:00"),
        ("2025-11-02T05:30:00Z", "2025-11-02T01:30:00"),
        ("2025-11-02T06:30:00Z", "2025-11-02T01:30:00"),
    ] {
        let expected: chrono::NaiveDateTime = local.parse().unwrap();
        assert_eq!(
            clock.at(date(at)).unwrap(),
            Date::DateTime {
                date: expected.date(),
                time: expected.time(),
                zone: "America/New_York".into()
            }
        );
    }
    let seoul = Clock::known("Asia/Seoul".parse().unwrap()).unwrap();
    assert_eq!(
        visible(
            &seoul
                .age(
                    Age::Date(date("2026-01-15T23:30:00Z")),
                    Translator::get(ENGLISH).unwrap()
                )
                .unwrap()
        ),
        "Jan 16"
    );
}
#[test]
fn unknown_viewer_is_explicit_utc_and_calendar_failures_are_localized() {
    let clock = Clock::unknown();
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    assert_eq!(clock.name(), None);
    assert!(!clock.is_known());
    assert_eq!(
        visible(
            &clock
                .age(Age::Date(date("2026-01-15T23:30:00Z")), en)
                .unwrap()
        ),
        "Jan 15 UTC"
    );
    for extreme in [DateTime::<Utc>::MIN_UTC, DateTime::<Utc>::MAX_UTC] {
        let error = clock.at(extreme).unwrap_err();
        assert_eq!(
            error.label(en),
            "This date is outside the calendar’s display range."
        );
        assert_eq!(error.label(ko), "표시할 수 있는 날짜 범위를 벗어났습니다.");
    }
    let error = Clock::known("Not/A_Zone".parse().unwrap()).unwrap_err();
    assert_eq!(
        error.label(en),
        "This time zone is unavailable. Update the app to display times in it."
    );
    assert_eq!(
        error.label(ko),
        "이 시간대를 사용할 수 없습니다. 시각을 표시하려면 앱을 업데이트하세요."
    );
}
#[test]
fn the_two_workspaces_pin_the_same_jiff_and_bundled_data() {
    // A node database update cannot silently leave the display on older rules.
    let native = include_str!("../../../../Cargo.lock");
    let node = include_str!("../../../../../../Cargo.lock");
    for package in ["jiff", "jiff-tzdb"] {
        let section = |lock: &str| {
            lock.split("[[package]]")
                .find(|part| {
                    part.lines()
                        .any(|line| line == format!("name = \"{package}\""))
                })
                .unwrap()
                .lines()
                .filter(|line| line.starts_with("version =") || line.starts_with("checksum ="))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(section(native), section(node));
    }
}

#[test]
fn each_display_failure_keeps_its_source_and_whole_words_in_both_languages() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    let errors = [
        (
            ClockError::Part(u32::try_from(-1_i8).unwrap_err()),
            true,
            "This date is outside the calendar’s display range.",
            "표시할 수 있는 날짜 범위를 벗어났습니다.",
        ),
        (
            ClockError::Calendar,
            false,
            "This date is outside the calendar’s display range.",
            "표시할 수 있는 날짜 범위를 벗어났습니다.",
        ),
        (
            ClockError::Format(FormatError::Missing("test-message")),
            true,
            "Some schedule details cannot be displayed. Update the app and try again.",
            "일부 예약 정보를 표시할 수 없습니다. 앱을 업데이트한 뒤 다시 시도하세요.",
        ),
    ];
    for (error, source, english, korean) in errors {
        assert_eq!(std::error::Error::source(&error).is_some(), source);
        assert_eq!(error.label(en), english);
        assert_eq!(error.label(ko), korean);
    }
}
