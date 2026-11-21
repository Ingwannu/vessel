//! Representative schedule data and regressions specific to the native list.
use super::*;
use vessel_proto::automation::ScheduleInfo;

pub(super) fn info() -> ScheduleInfo {
    serde_json::from_str(include_str!("../../tests/fixtures/schedules/info.json")).unwrap()
}

pub(super) fn visible(text: &str) -> String {
    text.chars()
        .filter(|ch| !matches!(ch, '\u{2068}' | '\u{2069}'))
        .collect()
}

#[test]
fn person_owned_session_schedules_only_lose_actions_when_they_are_heartbeats() {
    let mut schedule = info();
    let ScheduleTarget::Session { session } = schedule.definition.target else {
        panic!("fixture must target a session");
    };
    assert!(!is_heartbeat(&schedule));
    assert!(actions(&schedule, &authority::Authority::Unknown).contains(&Action::RunNow));
    schedule.defined_by = vessel_proto::Creator::Agent {
        node: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        session,
    };
    assert!(is_heartbeat(&schedule));
    assert_eq!(
        actions(&schedule, &authority::Authority::Unknown),
        [Action::Delete]
    );
}
