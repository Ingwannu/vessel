use super::{Draft, Save};
use crate::schedules::tests::info;
use vessel_proto::{
    Creator, HarnessKind,
    automation::{Cadence, CatchUp, ScheduleTarget},
    workspace::SessionPlace,
};

#[test]
fn edit_keeps_opened_revision_expiry_ownership_input_and_elapsed_phase() {
    let mut info = info();
    info.definition.catch_up = CatchUp::Skip;
    info.definition.ends_at = Some("2026-12-01T00:00:00Z".parse().unwrap());
    let mut draft = Draft::open(&info, None).unwrap();
    draft.fields.set_name("Updated name".to_owned());
    draft.cadence.select(super::cadence::Choice::Hour);
    draft.cadence.select(super::cadence::Choice::Interval);
    let Some(Save::Update(save)) = draft.save(draft.cadence.checked().unwrap()) else {
        panic!("expected revision-bound replacement");
    };
    assert_eq!(save.schedule, info.id);
    assert_eq!(save.revision, info.revision);
    assert_eq!(save.definition.ends_at, info.definition.ends_at);
    assert_eq!(save.definition.catch_up, CatchUp::Skip);
    assert_eq!(save.definition.target, info.definition.target);
    assert_eq!(save.definition.prompt, info.definition.prompt);
    assert_eq!(save.definition.cadence, info.definition.cadence);
}

#[test]
fn person_and_agent_heartbeats_cannot_make_a_definition_replacement() {
    let mut info = info();
    let session = "00000000-0000-4000-8000-000000000002".parse().unwrap();
    info.definition.target = ScheduleTarget::Session { session };
    assert!(Draft::open(&info, None).is_none());
    info.defined_by = Creator::Agent {
        session,
        node: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
    };
    assert!(Draft::open(&info, None).is_none());
}

#[test]
fn a_valid_zone_and_exact_run_cap_are_required_for_a_new_schedule() {
    let mut draft = Draft::create(None);
    assert!(draft.cadence.checked().is_err());
    draft
        .cadence
        .default_zone(Some(&"Asia/Seoul".parse().unwrap()));
    let Cadence::Cron { zone, .. } = draft.cadence.checked().unwrap() else {
        panic!("expected default hourly cron");
    };
    assert_eq!(zone.as_str(), "Asia/Seoul");
    draft.fields.set_prompt("Check recent changes".to_owned());
    draft.select_model(HarnessKind::ClaudeCode.into(), None);
    assert!(draft.save(draft.cadence.checked().unwrap()).is_none());
    draft.set_place(SessionPlace::Project {
        project: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        dir: None,
    });
    for invalid in ["0", "-5", "2.5", "4runs", "999999999999"] {
        draft.fields.set_limit(invalid.to_owned());
        assert!(draft.save(draft.cadence.checked().unwrap()).is_none());
    }
    draft.fields.set_limit("12".to_owned());
    let Some(Save::Create(save)) = draft.save(draft.cadence.checked().unwrap()) else {
        panic!("expected a valid schedule");
    };
    assert_eq!(save.definition.max_runs.unwrap().get(), 12);
}
