use super::*;
use crate::localization::{ENGLISH, Locale};
use crate::schedules::page::tests::loaded;
use crate::schedules::tests::visible;
use vessel_proto::automation::{RunError, RunList, RunTrigger};
#[test]
fn history_prepares_exact_title_time_outcome_and_link_outside_paint() {
    let (page, key) = loaded();
    let mut reader = Reader::default();
    let request = reader.open(&page.row(&key).unwrap()).unwrap();
    let session = "00000000-0000-4000-8000-000000000088".parse().unwrap();
    let at = "2026-11-15T00:00:00Z".parse().unwrap();
    assert!(reader.settle(
        &request,
        Ok(RunList {
            runs: vec![ScheduleRun {
                number: RunNumber::FIRST,
                at,
                due_at: at,
                trigger: RunTrigger::Due,
                outcome: RunOutcome::Started { session }
            }]
        })
    ));
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    let clock = Clock::known("UTC".parse().unwrap()).unwrap();
    let english = prepare(&reader, &clock, en).unwrap().unwrap();
    assert_eq!(
        visible(&english.title),
        "Run history · 빌드와 테스트 상태 확인"
    );
    assert_eq!(
        visible(&english.runs[0].heading),
        "Run 1 · Sun, Nov 15, 2026 at 12:00 AM UTC"
    );
    assert_eq!(english.runs[0].outcome.as_ref(), "Running");
    assert_eq!(english.runs[0].variant, Variant::Muted);
    assert_eq!(
        english.runs[0].link.as_ref().unwrap().connection(),
        request.connection()
    );
    assert_eq!(english.runs[0].link.as_ref().unwrap().session(), session);
    let korean = prepare(&reader, &clock, ko).unwrap().unwrap();
    assert_eq!(
        visible(&korean.title),
        "실행 기록 · 빌드와 테스트 상태 확인"
    );
    assert_eq!(korean.runs[0].outcome.as_ref(), "실행 중");
    assert_eq!(english, prepare(&reader, &clock, en).unwrap().unwrap());
}
#[test]
fn historical_errors_have_localized_meanings_without_private_diagnostics() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    let cases = [
        (
            RunErrorCode::InvalidRequest,
            "The machine rejected the run’s input.",
            "머신에서 이 실행의 입력을 거절했습니다.",
        ),
        (
            RunErrorCode::Unauthorized,
            "The credential used for this run was not accepted.",
            "이 실행에 사용한 인증 정보를 받아들이지 않았습니다.",
        ),
        (
            RunErrorCode::Forbidden,
            "The identity that created this schedule could not perform the run.",
            "예약을 만든 주체에게 이 실행을 수행할 권한이 없었습니다.",
        ),
        (
            RunErrorCode::ProtocolMismatch,
            "The run used an incompatible protocol.",
            "이 실행에서 호환되지 않는 통신 규약을 사용했습니다.",
        ),
        (
            RunErrorCode::NotFound,
            "A resource needed by this run could not be found.",
            "이 실행에 필요한 리소스를 찾지 못했습니다.",
        ),
        (
            RunErrorCode::Conflict,
            "Another operation prevented this run from starting.",
            "다른 작업 때문에 이 실행을 시작하지 못했습니다.",
        ),
        (
            RunErrorCode::Unsupported,
            "The machine did not support an operation needed by this run.",
            "머신에서 이 실행에 필요한 작업을 지원하지 않았습니다.",
        ),
        (
            RunErrorCode::HarnessNotInstalled,
            "The required agent was not installed.",
            "필요한 에이전트가 설치되어 있지 않았습니다.",
        ),
        (
            RunErrorCode::SpawnFailed,
            "The agent could not start.",
            "에이전트를 시작하지 못했습니다.",
        ),
        (
            RunErrorCode::Unavailable,
            "A service needed by this run was unavailable.",
            "이 실행에 필요한 서비스를 사용할 수 없었습니다.",
        ),
        (
            RunErrorCode::Internal,
            "The machine could not complete this run.",
            "머신에서 이 실행을 완료하지 못했습니다.",
        ),
        (
            RunErrorCode::Unknown,
            "Update the app to read this run’s reason.",
            "이 실행의 사유를 보려면 앱을 업데이트하세요.",
        ),
    ];
    for (code, english, korean) in cases {
        let outcome = RunOutcome::Failed {
            error: RunError {
                code,
                message: "private diagnostic never shown".into(),
            },
            session: None,
        };
        assert_eq!(reason(&outcome, en).unwrap().as_deref(), Some(english));
        assert_eq!(reason(&outcome, ko).unwrap().as_deref(), Some(korean));
    }
}
#[test]
fn every_typed_skip_reason_has_whole_bilingual_copy() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    for (why, english, korean) in [
        (
            SkipReason::PreviousRunning,
            "The previous run is still active.",
            "이전 실행이 아직 진행 중입니다.",
        ),
        (
            SkipReason::PreviousWaiting,
            "The previous prompt is still queued.",
            "이전 프롬프트가 아직 대기 중입니다.",
        ),
        (
            SkipReason::Stopped,
            "The chat was stopped and was not restarted automatically.",
            "대화를 중지한 상태여서 자동으로 다시 시작하지 않았습니다.",
        ),
        (
            SkipReason::Missed { count: 3 },
            "Missed 3 scheduled times while the machine was unavailable.",
            "머신에 연결할 수 없는 동안 예약된 실행 3회를 건너뛰었습니다.",
        ),
        (
            SkipReason::Unknown,
            "Update the app to read this run’s reason.",
            "이 실행의 사유를 보려면 앱을 업데이트하세요.",
        ),
    ] {
        let outcome = RunOutcome::Skipped { reason: why };
        assert_eq!(visible(&reason(&outcome, en).unwrap().unwrap()), english);
        assert_eq!(visible(&reason(&outcome, ko).unwrap().unwrap()), korean);
    }
}
#[test]
fn every_result_owns_a_badge_variant_without_a_second_failure_notice() {
    for (outcome, expected) in [
        (Outcome::Making, Variant::Muted),
        (Outcome::Running, Variant::Muted),
        (Outcome::Waiting, Variant::Warning),
        (Outcome::Finished, Variant::Success),
        (Outcome::Prompted, Variant::Success),
        (Outcome::Steered, Variant::Success),
        (Outcome::Queued, Variant::Muted),
        (Outcome::Skipped, Variant::Muted),
        (Outcome::Failed, Variant::Error),
        (Outcome::Cancelled, Variant::Muted),
        (Outcome::Limited, Variant::Warning),
        (Outcome::Refused, Variant::Error),
        (Outcome::SessionEnded, Variant::Muted),
        (Outcome::Interrupted, Variant::Muted),
        (Outcome::Unknown, Variant::Muted),
    ] {
        assert_eq!(badge(outcome), expected);
    }
}
