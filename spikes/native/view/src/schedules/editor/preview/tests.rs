use super::*;
use crate::localization::{ENGLISH, Locale};
use crate::schedules::load::{MachineKey, Tickets};
use crate::schedules::page::tests::{context, rebound};
use vessel_proto::automation::EverySeconds;
fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}
fn every(anchor: Option<DateTime<Utc>>) -> Cadence {
    Cadence::Every {
        seconds: EverySeconds::try_from(60).unwrap(),
        anchor,
    }
}
fn request() -> Request {
    Request::on(
        context(&MachineKey::of("laptop").unwrap()),
        Tickets::default().issue().unwrap(),
        every(None),
        at("2026-11-15T00:00:00Z"),
    )
    .unwrap()
}
fn response(request: &Request) -> SchedulePreview {
    SchedulePreview {
        after: request.input.after.unwrap(),
        cadence: every(request.input.after),
        runs: [
            "2026-11-15T00:01:00Z",
            "2026-11-15T00:02:00Z",
            "2026-11-15T00:03:00Z",
        ]
        .map(at)
        .to_vec(),
    }
}
fn accepts(request: Request, value: SchedulePreview) -> State {
    let mut state = State::default();
    state.begin(request.clone());
    assert!(state.settle(&request, Ok(value)));
    state
}
#[test]
fn only_matching_complete_node_projection_supplies_the_anchor_for_saving() {
    let request = request();
    let value = response(&request);
    let state = accepts(request.clone(), value.clone());
    assert_eq!(state.times(), value.runs);
    assert_eq!(
        state.canonical_for(request.connection(), &request.input.cadence),
        Some(&value.cadence)
    );
    assert!(
        state
            .canonical_for(
                &rebound(&MachineKey::of("laptop").unwrap()),
                &request.input.cadence
            )
            .is_none()
    );
    assert!(
        state
            .canonical_for(request.connection(), &Cadence::Unknown)
            .is_none()
    );
    assert!(state.problem().is_none());
    assert!(!state.pending());
}
#[test]
fn malformed_partial_unordered_or_different_cadence_never_shows_partial_times() {
    let request = request();
    let value = response(&request);
    let mut cases = Vec::new();
    let mut wrong = value.clone();
    wrong.after = at("2026-11-15T00:00:01Z");
    cases.push(("different-after", wrong));
    let mut wrong = value.clone();
    wrong.runs.pop();
    cases.push(("partial", wrong));
    let mut wrong = value.clone();
    wrong.runs.push(at("2026-11-15T00:04:00Z"));
    cases.push(("extra", wrong));
    let mut wrong = value.clone();
    wrong.runs.swap(0, 1);
    cases.push(("unsorted", wrong));
    let mut wrong = value.clone();
    wrong.runs[1] = wrong.runs[0];
    cases.push(("duplicate", wrong));
    let mut wrong = value.clone();
    wrong.runs[0] = value.after;
    cases.push(("not-strictly-after", wrong));
    let mut wrong = value.clone();
    wrong.cadence = every(None);
    cases.push(("missing-anchor", wrong));
    let mut wrong = value.clone();
    wrong.cadence = every(Some(at("2026-11-14T00:00:00Z")));
    cases.push(("wrong-anchor", wrong));
    let mut wrong = value.clone();
    wrong.cadence = Cadence::Unknown;
    cases.push(("unknown-cadence", wrong));
    let mut wrong = value;
    wrong.cadence = crate::schedules::cadence::cron("* * * * *", "UTC".parse().unwrap()).unwrap();
    cases.push(("different-kind", wrong));
    for (id, wrong) in cases {
        let state = accepts(request.clone(), wrong);
        assert!(state.times().is_empty(), "{id}");
        assert!(matches!(state.problem(), Some(Problem::Mismatched)), "{id}");
        assert!(
            state
                .canonical_for(request.connection(), &request.input.cadence)
                .is_none(),
            "{id}"
        );
    }
}
#[test]
fn typed_cron_expression_and_zone_must_both_match() {
    let mut request = request();
    request.input.cadence =
        crate::schedules::cadence::cron("* * * * *", "Asia/Seoul".parse().unwrap()).unwrap();
    let mut value = response(&request);
    value.cadence = request.input.cadence.clone();
    assert!(!accepts(request.clone(), value.clone()).times().is_empty());
    for (expression, zone) in [("* * * * *", "UTC"), ("0 * * * *", "Asia/Seoul")] {
        value.cadence = crate::schedules::cadence::cron(expression, zone.parse().unwrap()).unwrap();
        assert!(accepts(request.clone(), value.clone()).times().is_empty());
    }
}
#[test]
fn an_old_response_cannot_replace_a_new_edit_or_a_closed_form() {
    let old = request();
    let value = response(&old);
    let mut current = old.clone();
    let mut tickets = Tickets::default();
    let _first = tickets.issue().unwrap();
    current.ticket = tickets.issue().unwrap();
    let mut state = State::default();
    state.begin(current.clone());
    assert!(!state.settle(&old, Ok(value.clone())));
    assert!(state.pending());
    assert!(state.times().is_empty());
    assert!(state.settle(&current, Ok(value.clone())));
    assert!(!state.times().is_empty());
    state.invalidate();
    assert!(!state.settle(&current, Ok(value)));
    assert!(state.times().is_empty());
}
#[test]
fn unknown_preview_refusal_and_every_known_reason_have_both_languages() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    let cases = [
        (
            SchedulePreviewError::UnknownZone,
            "This machine does not know that time zone. Choose another zone or update the machine.",
            "이 머신에서 해당 시간대를 지원하지 않습니다. 다른 시간대를 선택하거나 머신을 업데이트하세요.",
        ),
        (
            SchedulePreviewError::UnknownCadence,
            "Update this machine before previewing this kind of schedule.",
            "이 종류의 예약을 미리 보려면 머신을 업데이트하세요.",
        ),
        (
            SchedulePreviewError::NoFutureRun,
            "This rule has no three future runs within the machine’s supported range.",
            "머신에서 지원하는 기간 안에 이 규칙으로 실행할 시각 세 개를 찾을 수 없습니다.",
        ),
        (
            SchedulePreviewError::TimeOutOfRange,
            "The selected time is outside this machine’s supported range.",
            "선택한 시각이 머신에서 지원하는 범위를 벗어납니다.",
        ),
        (
            SchedulePreviewError::Unknown,
            "Update the app to read this machine’s preview refusal.",
            "머신의 미리보기 거절 사유를 보려면 앱을 업데이트하세요.",
        ),
    ];
    for (reason, english, korean) in cases {
        let mut wire = vessel_proto::WireError::new(
            vessel_proto::ErrorCode::InvalidRequest,
            "private diagnostic never displayed",
        );
        wire.detail = Some(vessel_proto::ErrorDetail::SchedulePreview { reason });
        let mut state = State::default();
        let request = request();
        state.begin(request.clone());
        assert!(state.settle(&request, Err(Failure::Node(wire))));
        assert!(state.times().is_empty());
        let problem = state.problem().unwrap();
        assert_eq!(problem, &Problem::Rule(reason));
        assert_eq!(problem.label(en), english);
        assert_eq!(problem.label(ko), korean);
    }
    for (catalog, expected) in [
        (
            en,
            "The machine returned a different or incomplete preview. No run times are shown.",
        ),
        (
            ko,
            "머신에서 규칙과 다르거나 불완전한 미리보기를 받았습니다. 실행 시각을 표시할 수 없습니다.",
        ),
    ] {
        assert_eq!(Problem::Mismatched.label(catalog), expected);
        assert_eq!(
            Problem::Call(Failure::Older).label(catalog),
            catalog.schedules().preview_unavailable_older()
        );
    }
}
