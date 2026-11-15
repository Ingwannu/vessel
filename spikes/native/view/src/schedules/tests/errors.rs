//! Every local error variant gets a Korean and English UI result. Sources
//! remain inspectable for diagnostics; no UI relies on their Display wording.

use super::super::cadence::{CadenceError, CadenceInput, IntervalUnit};
use super::super::editor::EditError;
use crate::Lang;
use std::error::Error as _;

fn cron_error(expression: &str) -> CadenceError {
    CadenceInput::Cron {
        expression,
        zone: "UTC",
    }
    .parse()
    .unwrap_err()
}

#[test]
fn reference_grammar_failures_are_named_per_field_without_translating_source_prose() {
    // Paseo utils/schedule-format.test.ts: validateCron cases. Vessel
    // deliberately accepts named weekdays case-insensitively (its wire grammar).
    for (input, en, ko) in [
        ("", "Enter a cron expression", "예약 규칙을 입력하세요"),
        (
            "* * *",
            "Cron expressions must have 5 fields",
            "예약 규칙에는 항목이 5개 있어야 합니다",
        ),
        (
            "60 * * * *",
            "Invalid minute value",
            "분 값이 올바르지 않습니다",
        ),
        (
            "* 24 * * *",
            "Invalid hour value",
            "시간 값이 올바르지 않습니다",
        ),
        (
            "* * 31-1 * *",
            "Invalid day-of-month range",
            "날짜 범위가 올바르지 않습니다",
        ),
        (
            "* * * */0 *",
            "Invalid month step",
            "월 간격이 올바르지 않습니다",
        ),
        (
            "* * * * 8",
            "Invalid day-of-week value",
            "요일 값이 올바르지 않습니다",
        ),
        (
            "*/5/2 * * * *",
            "Invalid minute step",
            "분 간격이 올바르지 않습니다",
        ),
        (
            "@reboot",
            "Choose a supported cron shorthand",
            "지원하는 예약 규칙 축약형을 선택하세요",
        ),
    ] {
        let error = cron_error(input);
        assert_eq!(Lang::En.cadence_error(&error), en);
        assert_eq!(Lang::Ko.cadence_error(&error), ko);
        if !input.is_empty() {
            assert!(error.source().is_some());
        }
    }
    let long = cron_error(&"*".repeat(257));
    assert_eq!(
        Lang::En.cadence_error(&long),
        "Cron expressions must be at most 256 bytes"
    );
    assert_eq!(
        Lang::Ko.cadence_error(&long),
        "예약 규칙은 256바이트 이하여야 합니다"
    );
    assert!(
        CadenceInput::Cron {
            expression: "* * * * mon",
            zone: "UTC"
        }
        .parse()
        .is_ok()
    );
}

#[test]
fn interval_errors_use_the_selected_unit_and_retain_their_source() {
    for (unit, en, ko) in [
        (
            IntervalUnit::Seconds,
            "Enter 60 to 31622400 seconds",
            "60~31622400초 사이로 입력하세요",
        ),
        (
            IntervalUnit::Minutes,
            "Enter 1 to 527040 minutes",
            "1~527040분 사이로 입력하세요",
        ),
        (
            IntervalUnit::Hours,
            "Enter 1 to 8784 hours",
            "1~8784시간 사이로 입력하세요",
        ),
        (
            IntervalUnit::Days,
            "Enter 1 to 366 days",
            "1~366일 사이로 입력하세요",
        ),
    ] {
        for count in [
            "0",
            "-1",
            "4294967295",
            "18446744073709551616",
            "170141183460469231731687303715884105727",
            "340282366920938463463374607431768211456",
            "-340282366920938463463374607431768211456",
        ] {
            let error = CadenceInput::Every { count, unit }.parse().unwrap_err();
            assert_eq!(Lang::En.cadence_error(&error), en);
            assert_eq!(Lang::Ko.cadence_error(&error), ko);
            assert!(error.source().is_some());
        }
        for count in ["", "1.5", "abc"] {
            let error = CadenceInput::Every { count, unit }.parse().unwrap_err();
            assert_eq!(Lang::En.cadence_error(&error), "Enter a whole number");
            assert_eq!(Lang::Ko.cadence_error(&error), "정수를 입력하세요");
            assert!(error.source().is_some());
        }
    }
    let overflow = CadenceInput::Every {
        count: "4294967296",
        unit: IntervalUnit::Seconds,
    }
    .parse()
    .unwrap_err();
    assert!(matches!(overflow, CadenceError::Overflow { .. }));
    assert!(matches!(
        CadenceInput::Every {
            count: "0",
            unit: IntervalUnit::Seconds
        }
        .parse()
        .unwrap_err(),
        CadenceError::Range { .. }
    ));
}

#[test]
fn zone_and_edit_failures_speak_the_persons_language_without_log_copy() {
    let error = CadenceInput::Cron {
        expression: "0 9 * * *",
        zone: "../etc",
    }
    .parse()
    .unwrap_err();
    assert!(matches!(error, CadenceError::Zone(_)));
    assert!(error.source().is_some());
    assert_eq!(
        Lang::En.cadence_error(&error),
        "Enter an IANA time zone, such as Europe/London"
    );
    assert_eq!(
        Lang::Ko.cadence_error(&error),
        "Asia/Seoul 같은 IANA 시간대를 입력하세요"
    );
    for (error, en, ko) in [
        (
            EditError::Unknown,
            "This machine uses a newer format. Update the app to view it.",
            "머신에서 더 새로운 형식을 사용합니다. 앱을 업데이트하면 볼 수 있습니다.",
        ),
        (
            EditError::Ended,
            "An ended schedule cannot be edited",
            "끝난 예약은 수정할 수 없습니다",
        ),
        (
            EditError::SessionMode,
            "A schedule starts a new chat each time; it cannot continue an old one or open a terminal session",
            "예약은 실행할 때마다 새 대화를 시작합니다. 이전 대화를 이어서 시작하거나 터미널 세션을 열 수는 없습니다.",
        ),
    ] {
        assert_eq!(Lang::En.edit_error(error), en);
        assert_eq!(Lang::Ko.edit_error(error), ko);
        assert_ne!(error.to_string(), en);
        assert_ne!(error.to_string(), ko);
    }
}
