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
    draft.cadence.default_zone(Some(&"Asia/Seoul".parse().unwrap()));
    draft.cadence.set_expression(String::new());
    assert!(draft.cadence.is_elapsed());
    let locale = crate::localization::Translator::get(crate::localization::ENGLISH).unwrap();
    assert_eq!(super::cadence::description(draft.cadence.checked().unwrap(), locale).unwrap(), "Every 90 seconds");
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

#[test]
fn an_older_elapsed_forecast_cannot_settle_a_new_cron_rule() {
    use super::io::{Ask, Form, Targets, Value};
    use crate::{
        connection::{ConnectionEpochs, MachineKey},
        schedules::load::Tickets,
    };
    use chrono::{DateTime, TimeDelta};
    use vessel_proto::{
        Scope,
        automation::{PreviewCount, PreviewSchedule, SchedulePreview},
    };

    let connection = ConnectionEpochs::default()
        .admit(
            MachineKey::from_book("laptop"),
            vessel_proto::NodeId::random(),
        )
        .unwrap();
    let mut form = Form::new(connection, Draft::open(&info(), None).unwrap());
    let mut tickets = Tickets::default();
    let targets = form.issue(&mut tickets, Ask::Targets).unwrap();
    assert!(form.accept(
        &targets,
        Ok(Value::Targets(Box::new(Targets {
            projects: Vec::new(),
            worktrees: Vec::new(),
            harnesses: Vec::new(),
            agents: Vec::new(),
            scopes: Ok(vec![Scope::Operate]),
            machine_zone: Some("Asia/Seoul".parse().unwrap()),
            device_zone: Some("Europe/London".parse().unwrap()),
        })))
    ));
    assert_eq!(form.draft.cadence.zone(), "Europe/London");
    let after = DateTime::UNIX_EPOCH;
    let input = |cadence| {
        Ask::Preview(PreviewSchedule {
            cadence,
            after: Some(after),
            count: PreviewCount::try_from(1).unwrap(),
        })
    };
    let old_rule = form.draft.cadence.checked().unwrap().clone();
    let old = form.issue(&mut tickets, input(old_rule)).unwrap();
    form.draft.cadence.select(super::cadence::Choice::Daily);
    form.invalidate_preview();
    let new_rule = form.draft.cadence.checked().unwrap().clone();
    let current = form.issue(&mut tickets, input(new_rule.clone())).unwrap();
    let forecast = SchedulePreview {
        after,
        cadence: new_rule.clone(),
        runs: vec![after + TimeDelta::days(1)],
    };
    assert!(!form.accept(&old, Ok(Value::Preview(Box::new(forecast.clone())))));
    assert!(!form.can_save());
    assert!(form.accept(&current, Ok(Value::Preview(Box::new(forecast)))));
    let Some(Save::Update(save)) = form.save_request() else {
        panic!("expected a new cron rule with a matching forecast");
    };
    assert_eq!(save.definition.cadence, new_rule);
    assert!(matches!(save.definition.cadence, Cadence::Cron { .. }));
}

#[test]
fn an_explicit_cron_expression_replaces_the_stored_elapsed_rule() {
    let mut draft = Draft::open(&info(), Some(&"Asia/Seoul".parse().unwrap())).unwrap();
    assert!(draft.cadence.is_elapsed());
    draft.cadence.set_expression("17 8 * * *".to_owned());
    let Cadence::Cron { expr, zone } = draft.cadence.checked().unwrap() else {
        panic!("expected cron");
    };
    assert_eq!(expr.as_str(), "17 8 * * *");
    assert_eq!(zone.as_str(), "Asia/Seoul");
}
