//! Portable contract cases: the page has no schedules tests. These JSON inputs
//! and expected decisions are deliberately independent of GPUI and Rust enums.

use super::authority::Authority;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use vessel_proto::automation::{EverySeconds, RunOutcome, ScheduleInfo, ScheduleState};
use vessel_proto::{Scope, SessionId, SessionSpec};

mod cases;
mod errors;
mod formatting;
use super::*;
use cases::{ExpectedAction, ExpectedBadge, ExpectedBucket, ExpectedOutcome};

pub(super) fn info() -> ScheduleInfo {
    serde_json::from_str(include_str!("../../../tests/fixtures/schedules/info.json")).unwrap()
}

#[derive(Deserialize)]
struct StateCase {
    state: ScheduleState,
    bucket: ExpectedBucket,
    badge: ExpectedBadge,
    actions: Vec<ExpectedAction>,
    en: String,
    ko: String,
}

#[test]
fn node_states_define_buckets_actions_and_next_due() {
    let cases: Vec<StateCase> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/schedules/states.json"
    ))
    .unwrap();
    for case in cases {
        let schedule = ScheduleInfo {
            state: case.state,
            ..info()
        };
        assert_eq!(Status::of(&schedule.state).bucket(), case.bucket.into());
        assert_eq!(Status::of(&schedule.state).badge(), case.badge.into());
        let expected: Vec<Action> = case.actions.into_iter().map(Into::into).collect();
        assert_eq!(
            actions(
                &schedule,
                &Authority::Scopes(vec![Scope::Read, Scope::Operate])
            ),
            expected
        );
        assert!(actions(&schedule, &Authority::Scopes(vec![Scope::Read])).is_empty());
        assert!(actions(&schedule, &Authority::Scopes(vec![Scope::Admin])).is_empty());
        assert!(actions(&schedule, &Authority::Scopes(vec![])).is_empty());
        assert_eq!(
            crate::Lang::En
                .schedules()
                .status(Status::of(&schedule.state)),
            case.en
        );
        assert_eq!(
            crate::Lang::Ko
                .schedules()
                .status(Status::of(&schedule.state)),
            case.ko
        );
        assert_eq!(
            next_due(&schedule).is_some(),
            matches!(schedule.state, ScheduleState::Active { .. })
        );
    }
}

#[derive(Deserialize)]
struct OutcomeCase {
    value: RunOutcome,
    expected: ExpectedOutcome,
    session: Option<SessionId>,
    en: String,
    ko: String,
}

#[test]
fn every_run_shape_keeps_its_result_and_session_link() {
    let cases: Vec<OutcomeCase> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/schedules/outcomes.json"
    ))
    .unwrap();
    for case in cases {
        assert_eq!(history::outcome(&case.value), case.expected.into());
        assert_eq!(history::session(&case.value), case.session);
        assert_eq!(
            crate::Lang::En
                .schedules()
                .outcome(history::outcome(&case.value)),
            case.en
        );
        assert_eq!(
            crate::Lang::Ko
                .schedules()
                .outcome(history::outcome(&case.value)),
            case.ko
        );
    }
}

#[test]
fn title_is_name_then_session_title_then_first_nonblank_prompt_line() {
    let mut schedule = info();
    assert_eq!(title(&schedule.definition), "빌드와 테스트 상태 확인");
    let mut spec: SessionSpec = serde_json::from_str(
        r#"{"harness":"claude-code", "mode":"structured", "title":"Session title"}"#,
    )
    .unwrap();
    schedule.definition.target = ScheduleTarget::NewSession {
        spec: Box::new(spec.clone()),
        after: AfterRun::Archive,
    };
    assert_eq!(title(&schedule.definition), "Session title");
    let long_name = "매우 긴 이름 ".repeat(8).trim().to_owned();
    schedule.definition.name = Some(long_name.parse().unwrap());
    assert_eq!(title(&schedule.definition), long_name);
    schedule.definition.name = None;
    spec.title = None;
    schedule.definition.target = ScheduleTarget::NewSession {
        spec: Box::new(spec),
        after: AfterRun::Archive,
    };
    assert_eq!(title(&schedule.definition), "빌드와 테스트 상태 확인");
}

#[test]
fn row_order_is_stable_and_paused_rows_stay_under_active() {
    let first = info();
    let mut second = first.clone();
    second.id = "00000000-0000-4000-8000-000000000003".parse().unwrap();
    second.state = ScheduleState::Paused {
        since: second.created_at,
    };
    let input = [second.clone(), first.clone()];
    assert_eq!(rows(&input, Bucket::Active), [&first, &second]);
    assert!(rows(&input, Bucket::Ended).is_empty());
    let later: DateTime<Utc> = "2026-11-14T00:00:00Z".parse().unwrap();
    second.created_at = later;
    let input = [first.clone(), second.clone()];
    assert_eq!(rows(&input, Bucket::Active), [&second, &first]);
}

#[test]
fn cadence_is_parsed_once_without_rounding_or_resetting_phase_on_save() {
    use editor::CadenceInput;
    for seconds in [60, 90, EverySeconds::MAX] {
        let text = seconds.to_string();
        assert_eq!(
            CadenceInput::Every {
                count: &text,
                unit: editor::IntervalUnit::Seconds
            }
            .parse()
            .unwrap(),
            Cadence::Every {
                seconds: EverySeconds::try_from(seconds).unwrap(),
                anchor: None
            }
        );
    }
    for text in ["59", "31622401", "0", "-1", "1.5", "", "9999999999999"] {
        assert!(
            CadenceInput::Every {
                count: text,
                unit: editor::IntervalUnit::Seconds
            }
            .parse()
            .is_err(),
            "{text}"
        );
    }
    let parsed = CadenceInput::Cron {
        expression: "0   9 * * MON-FRI",
        zone: "Asia/Seoul",
    }
    .parse()
    .unwrap();
    let Cadence::Cron { expr, zone } = parsed else {
        panic!("cron expected")
    };
    assert_eq!(expr.as_str(), "0 9 * * MON-FRI");
    assert_eq!(zone.as_str(), "Asia/Seoul");
    for (expression, zone) in [("0 0 9 * * *", "UTC"), ("0 9 * * *", "../etc")] {
        assert!(CadenceInput::Cron { expression, zone }.parse().is_err());
    }
    let opened = info();
    let update = editor::replacement(&opened, opened.definition.clone()).unwrap();
    assert_eq!(update.revision, 7);
    assert_eq!(update.definition.cadence, opened.definition.cadence);
    assert_eq!(update.schedule, opened.id);
    assert_eq!(
        crate::Lang::En.schedule_cadence(&update.definition.cadence),
        "Every 1 minute 30 seconds"
    );
    assert_eq!(
        crate::Lang::Ko.schedule_cadence(&update.definition.cadence),
        "1분 30초마다"
    );
}

#[test]
fn unknown_definition_states_and_reasons_cannot_be_overwritten() {
    let schedule = info();
    let mut variants = Vec::new();
    let mut definition = schedule.definition.clone();
    definition.cadence = Cadence::Unknown;
    variants.push(definition);
    let mut definition = schedule.definition.clone();
    definition.catch_up = CatchUp::Unknown;
    variants.push(definition);
    let mut definition = schedule.definition.clone();
    definition.target = ScheduleTarget::Unknown;
    variants.push(definition);
    let mut definition = schedule.definition.clone();
    let spec: SessionSpec =
        serde_json::from_str(r#"{"harness":"claude-code", "mode":"structured"}"#).unwrap();
    definition.target = ScheduleTarget::NewSession {
        spec: Box::new(spec),
        after: AfterRun::Unknown,
    };
    variants.push(definition);
    for definition in variants {
        assert_eq!(
            editor::check_definition(&definition),
            Err(editor::EditError::Unknown)
        );
        let newer = ScheduleInfo {
            definition,
            ..schedule.clone()
        };
        assert!(!actions(&newer, &Authority::Scopes(vec![Scope::Operate])).contains(&Action::Edit));
        assert_eq!(
            editor::replacement(&newer, schedule.definition.clone()),
            Err(editor::EditError::Unknown)
        );
    }
    for state in [
        ScheduleState::Unknown,
        ScheduleState::Ended {
            at: schedule.created_at,
            reason: EndReason::Unknown,
        },
    ] {
        let ended = ScheduleInfo {
            state,
            ..schedule.clone()
        };
        assert_eq!(
            editor::replacement(&ended, ended.definition.clone()),
            Err(editor::EditError::Unknown)
        );
        assert!(!actions(&ended, &Authority::Scopes(vec![Scope::Operate])).contains(&Action::Edit));
    }
}

#[test]
fn automatic_new_sessions_refuse_terminal_and_resume_specs() {
    for json in [
        r#"{"harness":"claude-code", "mode":"terminal"}"#,
        r#"{"harness":"claude-code", "mode":"structured", "resume":"old-session"}"#,
    ] {
        let mut definition = info().definition;
        definition.target = ScheduleTarget::NewSession {
            spec: Box::new(serde_json::from_str(json).unwrap()),
            after: AfterRun::Archive,
        };
        assert_eq!(
            editor::check_definition(&definition),
            Err(editor::EditError::SessionMode)
        );
    }
}

// Reference fixtures describe visible words; keep separate exact byte checks
// for Fluent's directional isolates instead of changing production formatting.
pub(super) fn visible(text: &str) -> String {
    text.chars()
        .filter(|ch| !matches!(ch, '\u{2068}' | '\u{2069}'))
        .collect()
}
