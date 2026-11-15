//! Paseo utils/schedule-format.test.ts inputs and expected English results,
//! extended with exact seconds, Korean, aliases and date-restriction guards.

use super::super::cadence::{CadenceInput, IntervalParts, IntervalUnit};
use super::super::{Action, Product, ScheduleTarget, product};
use crate::Lang;
use serde::Deserialize;
use vessel_proto::automation::{Cadence, EverySeconds};

#[derive(Deserialize)]
struct FormatCase {
    source_case: String,
    cadence: Cadence,
    en: String,
    ko: String,
}

#[test]
fn reference_format_cases_and_exact_interval_extensions() {
    let cases: Vec<FormatCase> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/schedules/format.json"
    ))
    .unwrap();
    for case in cases {
        assert_eq!(
            Lang::En.schedule_cadence(&case.cadence),
            case.en,
            "{} {:?}",
            case.source_case,
            case.cadence
        );
        assert_eq!(
            Lang::Ko.schedule_cadence(&case.cadence),
            case.ko,
            "{} {:?}",
            case.source_case,
            case.cadence
        );
    }
}

#[test]
fn interval_form_picks_a_lossless_unit_and_round_trips_all_boundaries() {
    // The reference's everyMsToParts rounds 90 s to two minutes; this
    // form intentionally preserves ninety seconds, per the node contract.
    for (seconds, count, unit) in [
        (60, 1, IntervalUnit::Minutes),
        (90, 90, IntervalUnit::Seconds),
        (7200, 2, IntervalUnit::Hours),
        (172_800, 2, IntervalUnit::Days),
        (61, 61, IntervalUnit::Seconds),
        (EverySeconds::MAX, 366, IntervalUnit::Days),
    ] {
        let original = EverySeconds::try_from(seconds).unwrap();
        let parts = IntervalParts::of(original);
        assert_eq!((parts.count(), parts.unit()), (count, unit));
        assert_eq!(
            CadenceInput::Every {
                count: &count.to_string(),
                unit
            }
            .parse()
            .unwrap(),
            Cadence::Every {
                seconds: original,
                anchor: None
            }
        );
    }
    for unit in IntervalUnit::DESCENDING {
        let (min, max) = unit.limits();
        for count in [min, max] {
            assert!(
                CadenceInput::Every {
                    count: &count.to_string(),
                    unit
                }
                .parse()
                .is_ok()
            );
        }
        for count in [min.saturating_sub(1), max.checked_add(1).unwrap()] {
            assert!(
                CadenceInput::Every {
                    count: &count.to_string(),
                    unit
                }
                .parse()
                .is_err()
            );
        }
    }
}

#[test]
fn product_words_follow_the_target_in_both_titles_and_actions() {
    assert_eq!(
        product(&super::info().definition.target),
        Product::RecurringPrompt
    );
    let spec = serde_json::from_str(r#"{"harness":"claude-code","mode":"structured"}"#).unwrap();
    assert_eq!(
        product(&ScheduleTarget::NewSession {
            spec,
            after: super::super::AfterRun::Archive
        }),
        Product::Schedule
    );
    assert_eq!(product(&ScheduleTarget::Unknown), Product::Unknown);
    for (kind, en, ko) in [
        (Product::Schedule, "Schedule", "예약"),
        (
            Product::RecurringPrompt,
            "Recurring prompt",
            "반복 프롬프트",
        ),
        (Product::Unknown, "Automation", "자동 실행"),
    ] {
        assert_eq!(Lang::En.schedule_product(kind), en);
        assert_eq!(Lang::Ko.schedule_product(kind), ko);
    }
    for (action, en, ko) in [
        (Action::Edit, "Edit recurring prompt", "반복 프롬프트 수정"),
        (
            Action::Pause,
            "Pause recurring prompt",
            "반복 프롬프트 일시 정지",
        ),
        (
            Action::Resume,
            "Resume recurring prompt",
            "반복 프롬프트 재개",
        ),
        (Action::RunNow, "Run now", "지금 실행"),
        (
            Action::Delete,
            "Delete recurring prompt",
            "반복 프롬프트 삭제",
        ),
    ] {
        assert_eq!(
            Lang::En.schedule_action(Product::RecurringPrompt, action),
            en
        );
        assert_eq!(
            Lang::Ko.schedule_action(Product::RecurringPrompt, action),
            ko
        );
    }
    assert_eq!(Lang::En.schedules().unknown_filter, "Other states");
    assert_eq!(Lang::Ko.schedules().unknown_filter, "기타 상태");
    assert_ne!(
        Lang::En.schedules().unknown_badge,
        Lang::En.schedules().newer_machine
    );
    assert_ne!(
        Lang::Ko.schedules().unknown_badge,
        Lang::Ko.schedules().newer_machine
    );
    assert_eq!(
        Lang::En.schedule_cadence(&Cadence::Unknown),
        Lang::En.schedules().newer_machine
    );
    assert_eq!(
        Lang::Ko.schedule_cadence(&Cadence::Unknown),
        Lang::Ko.schedules().newer_machine
    );
}

#[test]
fn schedule_and_unknown_target_action_words_stay_distinct_from_recurring_prompts() {
    for (action, en, ko) in [
        (Action::Edit, "Edit schedule", "예약 수정"),
        (Action::Pause, "Pause schedule", "예약 일시 정지"),
        (Action::Resume, "Resume schedule", "예약 재개"),
        (Action::RunNow, "Run now", "지금 실행"),
        (Action::Delete, "Delete schedule", "예약 삭제"),
    ] {
        assert_eq!(Lang::En.schedule_action(Product::Schedule, action), en);
        assert_eq!(Lang::Ko.schedule_action(Product::Schedule, action), ko);
    }
    for (action, en, ko) in [
        (Action::Edit, "Edit automation", "자동 실행 수정"),
        (Action::Pause, "Pause automation", "자동 실행 일시 정지"),
        (Action::Resume, "Resume automation", "자동 실행 재개"),
        (Action::RunNow, "Run now", "지금 실행"),
        (Action::Delete, "Delete automation", "자동 실행 삭제"),
    ] {
        assert_eq!(Lang::En.schedule_action(Product::Unknown, action), en);
        assert_eq!(Lang::Ko.schedule_action(Product::Unknown, action), ko);
    }
}

#[test]
fn interval_unit_choices_only_expose_seconds_to_preserve_an_opened_value() {
    use crate::schedules::cadence::IntervalUnit::{self, Days, Hours, Minutes, Seconds};
    let ordinary = &[Minutes, Hours, Days];
    assert_eq!(IntervalUnit::choices(None), ordinary);
    for value in [60, 120, 7200, 86400] {
        assert_eq!(
            IntervalUnit::choices(Some(value.try_into().unwrap())),
            ordinary
        );
    }
    for value in [90, 61, 3601] {
        assert_eq!(
            IntervalUnit::choices(Some(value.try_into().unwrap())),
            &[Seconds, Minutes, Hours, Days]
        );
    }
}
