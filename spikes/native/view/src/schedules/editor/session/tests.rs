//! Pure form-visit cases; no widget identity or transport permission is inferred.
use super::*;
use crate::schedules::editor::{
    cadence::Preset,
    definition::{Finish, Issue, NewTarget, Save},
};
use crate::schedules::load::MachineKey;
use crate::schedules::page::tests::{context, rebound};
use crate::schedules::tests::info;
use chrono::{DateTime, Utc};
use vessel_proto::automation::{EverySeconds, SchedulePreview};

mod choices;
mod retry;
fn at() -> DateTime<Utc> {
    "2026-11-15T00:00:00Z".parse().unwrap()
}
fn connection() -> ConnectionContext {
    context(&MachineKey::of("laptop").unwrap())
}
fn cadence() -> Cadence {
    Cadence::Every {
        seconds: EverySeconds::try_from(60).unwrap(),
        anchor: None,
    }
}
fn target() -> NewTarget {
    NewTarget::new(serde_json::from_str(r#"{"harness":"claude-code","mode":"structured","cwd":"/project","model":"old-model","labels":{"keep":"metadata"}}"#).unwrap(), Finish::Keep).unwrap()
}
fn active() -> (Model, Opened) {
    let mut model = Model::default();
    let visit = model
        .create(cadence(), TimeZoneName::utc())
        .unwrap()
        .unwrap();
    let opened = model.select_host(visit, connection()).unwrap();
    assert!(model.field(opened.visit, FieldChange::Prompt("Check the build".into())));
    assert!(model.target(opened.visit, target()));
    (model, opened)
}
fn response(request: &io::Request) -> io::Value {
    let io::Operation::Preview(input) = request.operation() else {
        panic!("expected preview")
    };
    let Cadence::Every { seconds, anchor } = &input.cadence else {
        panic!("expected interval")
    };
    io::Value::Preview(SchedulePreview {
        after: input.after.unwrap(),
        cadence: Cadence::Every {
            seconds: *seconds,
            anchor: Some(anchor.unwrap_or(input.after.unwrap())),
        },
        runs: [
            "2026-11-15T00:01:00Z",
            "2026-11-15T00:02:00Z",
            "2026-11-15T00:03:00Z",
        ]
        .map(|text| text.parse().unwrap())
        .to_vec(),
    })
}
fn load_options(model: &mut Model, visit: Visit) {
    if let Some(request) = model
        .mailbox(&io::Operation::Targets)
        .and_then(io::Mailbox::request)
        .cloned()
    {
        let _received = model.received(
            &request,
            Ok(io::Value::Targets(Box::new(choices::targets()))),
        );
    }
    let request = model.agent(visit, "claude-code".parse().unwrap()).unwrap();
    assert!(matches!(
        model.received(&request, Ok(io::Value::Options(choices::options()))),
        Received::Changed
    ));
}
fn ready(model: &mut Model, visit: Visit) -> Save {
    load_options(model, visit);
    let request = model.preview(visit, at()).unwrap().unwrap();
    assert!(matches!(
        model.received(&request, Ok(response(&request))),
        Received::Changed
    ));
    model.prepare_save().unwrap().unwrap()
}
#[test]
fn switching_host_preserves_text_but_retires_choices_visit_and_old_answers() {
    let (mut model, first) = active();
    assert!(model.field(first.visit, FieldChange::Name("한글 예약".into())));
    let preview = model.preview(first.visit, at()).unwrap().unwrap();
    let other = context(&MachineKey::of("other").unwrap());
    let next = model.select_host(first.visit, other.clone()).unwrap();
    assert_ne!(first.visit, next.visit);
    assert_eq!(model.connection(), Some(&other));
    assert!(model.draft().unwrap().target().is_none());
    assert!(model.projected().is_none());
    assert!(!model.field(first.visit, FieldChange::Name("late".into())));
    assert!(!model.close(first.visit));
    assert!(matches!(
        model.received(&preview, Ok(response(&preview))),
        Received::Ignored
    ));
    assert!(model.select_host(next.visit, other).is_none());
    assert!(model.target(next.visit, target()));
    let Save::Create(save) = ready(&mut model, next.visit) else {
        panic!("create")
    };
    assert_eq!(save.definition.name.unwrap().as_str(), "한글 예약");
    assert_eq!(save.definition.prompt.as_str(), "Check the build");
}
#[test]
fn save_reuses_node_phase_and_rejects_an_edited_or_old_candidate() {
    let (mut model, opened) = active();
    assert!(matches!(model.prepare_save(), Err(Issue::Preview)));
    let candidate = ready(&mut model, opened.visit);
    let Save::Create(save) = &candidate else {
        panic!("create")
    };
    assert_eq!(
        save.definition.cadence,
        Cadence::Every {
            seconds: EverySeconds::try_from(60).unwrap(),
            anchor: Some(at())
        }
    );
    assert!(model.field(opened.visit, FieldChange::Name("Changed".into())));
    assert!(model.save(opened.visit, candidate).is_none());
    let candidate = model.prepare_save().unwrap().unwrap();
    let request = model.save(opened.visit, candidate.clone()).unwrap();
    assert!(model.pending_save());
    assert!(model.save(opened.visit, candidate).is_none());
    assert!(!model.field(opened.visit, FieldChange::Prompt("late".into())));
    assert!(!model.close(opened.visit));
    assert!(
        model
            .create(cadence(), TimeZoneName::utc())
            .unwrap()
            .is_none()
    );
    let saved = Box::new(info());
    assert!(
        matches!(model.received(&request, Ok(io::Value::Saved(saved))), Received::Saved { visit, .. } if visit == opened.visit)
    );
    assert!(model.visit().is_none());
    assert!(matches!(
        model.received(&request, Ok(io::Value::Saved(Box::new(info())))),
        Received::Ignored
    ));
}
#[test]
fn editing_cadence_invalidates_pending_projection_and_old_reply() {
    let (mut model, opened) = active();
    let first = model.preview(opened.visit, at()).unwrap().unwrap();
    assert!(model.cadence(opened.visit, |draft| draft.preset(Preset::Weekdays)));
    assert!(matches!(
        model.received(&first, Ok(response(&first))),
        Received::Ignored
    ));
    assert!(model.projected().is_none());
    assert!(matches!(model.prepare_save(), Err(Issue::Preview)));
}
#[test]
fn older_preview_is_explicit_fallback_but_other_failures_never_authorize_save() {
    for failure in [
        Failure::Busy,
        Failure::TimedOut,
        Failure::Unexpected,
        Failure::Older,
    ] {
        let (mut model, opened) = active();
        load_options(&mut model, opened.visit);
        let request = model.preview(opened.visit, at()).unwrap().unwrap();
        assert!(matches!(
            model.received(&request, Err(failure.clone())),
            Received::Changed
        ));
        assert!(model.projected().is_none());
        assert_eq!(model.prepare_save().is_ok(), failure == Failure::Older);
    }
}
#[test]
fn same_node_reconnect_clears_reads_and_retains_uncertain_save_without_replay() {
    let (mut model, opened) = active();
    let save = ready(&mut model, opened.visit);
    let request = model.save(opened.visit, save.clone()).unwrap();
    assert!(!model.connected(&MachineKey::of("other").unwrap()));
    assert!(model.connected(&MachineKey::of("laptop").unwrap()));
    assert!(model.blocked());
    assert!(!model.pending_save());
    assert_eq!(
        model.mailbox(request.operation()).unwrap().failure(),
        Some(&Failure::Unconfirmed)
    );
    assert!(!model.connected(&MachineKey::of("laptop").unwrap()));
    assert!(!model.reconcile(&crate::Fleet {
        machines: vec![],
        read_at: None
    }));
    assert!(model.save(opened.visit, save).is_none());
    assert!(matches!(
        model.received(&request, Ok(io::Value::Saved(Box::new(info())))),
        Received::Ignored
    ));
    assert!(model.close(opened.visit));
}
#[test]
fn physical_replacement_and_close_reopen_refuse_old_read_and_input() {
    let (mut model, opened) = active();
    let preview = model.preview(opened.visit, at()).unwrap().unwrap();
    let wrong = io::Request::on(
        rebound(&MachineKey::of("laptop").unwrap()),
        preview.ticket(),
        preview.operation().clone(),
    );
    assert!(matches!(
        model.received(&wrong, Ok(response(&preview))),
        Received::Ignored
    ));
    assert!(model.close(opened.visit));
    let next = model
        .create(cadence(), TimeZoneName::utc())
        .unwrap()
        .unwrap();
    assert_ne!(next, opened.visit);
    assert!(!model.field(opened.visit, FieldChange::Prompt("stale".into())));
    assert!(matches!(
        model.received(&preview, Ok(response(&preview))),
        Received::Ignored
    ));
}
#[test]
fn recurring_edit_has_fixed_machine_and_only_cadence_changes() {
    let mut model = Model::default();
    let opened = model
        .edit(info(), connection(), TimeZoneName::utc())
        .unwrap()
        .unwrap();
    assert!(!model.field(opened.visit, FieldChange::Name("forbidden".into())));
    assert!(!model.target(opened.visit, target()));
    assert!(
        model
            .select_host(opened.visit, rebound(&MachineKey::of("laptop").unwrap()))
            .is_none()
    );
    let request = model.preview(opened.visit, at()).unwrap().unwrap();
    assert!(matches!(
        model.received(&request, Err(Failure::Older)),
        Received::Changed
    ));
    let Save::Update(save) = model.prepare_save().unwrap().unwrap() else {
        panic!("update")
    };
    assert_eq!(save.definition, info().definition);
    assert_eq!(save.revision, info().revision);
    assert_eq!(save.schedule, info().id);
}
#[test]
fn actual_forbidden_blocks_editing_and_retry_for_the_remaining_visit() {
    let (mut model, opened) = active();
    let candidate = ready(&mut model, opened.visit);
    let request = model.save(opened.visit, candidate).unwrap();
    assert!(matches!(
        model.received(
            &request,
            Err(Failure::Node(vessel_proto::WireError::new(
                vessel_proto::ErrorCode::Forbidden,
                "diagnostic"
            )))
        ),
        Received::Changed
    ));
    assert!(model.blocked());
    assert!(!model.field(opened.visit, FieldChange::Name("late".into())));
    assert!(model.preview(opened.visit, at()).unwrap().is_none());
    assert!(model.close(opened.visit));
}

#[test]
fn retirement_without_a_write_removes_choices_and_cannot_rebind_the_visit() {
    let (mut model, opened) = active();
    assert!(model.reconcile(&crate::Fleet {
        machines: vec![],
        read_at: None
    }));
    assert!(model.targets().is_none());
    assert!(model.projected().is_none());
    assert!(model.blocked());
    assert!(model.preview(opened.visit, at()).unwrap().is_none());
    assert!(
        model
            .select_host(opened.visit, rebound(&MachineKey::of("laptop").unwrap()))
            .is_none()
    );
    assert!(!model.reconcile(&crate::Fleet {
        machines: vec![],
        read_at: None
    }));
    assert!(model.close(opened.visit));
}

#[test]
fn known_grants_and_actual_refusal_are_preserved_without_guessing_missing_scopes() {
    for authority in [
        Authority::Unknown,
        Authority::Scopes(vec![vessel_proto::Scope::Read]),
        Authority::Scopes(vec![
            vessel_proto::Scope::Read,
            vessel_proto::Scope::Operate,
        ]),
    ] {
        let (mut model, opened) = active();
        let _changed = model.reported_authority(opened.visit, &authority);
        assert_eq!(model.blocked(), !authority.allows_actions());
        assert_eq!(model.authority(), Some(&authority));
        assert!(!model.reported_authority(opened.visit, &Authority::Unknown));
        assert_eq!(model.authority(), Some(&authority));
    }
    let (mut model, opened) = active();
    let candidate = ready(&mut model, opened.visit);
    let request = model.save(opened.visit, candidate).unwrap();
    assert!(matches!(
        model.received(
            &request,
            Err(Failure::Node(vessel_proto::WireError::new(
                vessel_proto::ErrorCode::Forbidden,
                "private diagnostic"
            )))
        ),
        Received::Changed
    ));
    assert_eq!(model.authority(), Some(&Authority::OperationForbidden));
    assert!(!model.reported_authority(
        opened.visit,
        &Authority::Scopes(vec![vessel_proto::Scope::Operate])
    ));
    assert_eq!(
        model.authority().unwrap().explanation(crate::Lang::En),
        Some("This device can only view this machine's schedules.")
    );
    assert_eq!(
        model.authority().unwrap().explanation(crate::Lang::Ko),
        Some("이 머신의 예약은 이 기기에서 조회만 할 수 있습니다.")
    );
}
#[test]
fn an_old_visit_cannot_change_grants_or_the_archive_switch() {
    let (mut model, opened) = active();
    let next = model
        .select_host(opened.visit, rebound(&MachineKey::of("laptop").unwrap()))
        .unwrap();
    assert!(!model.reported_authority(opened.visit, &Authority::Scopes(vec![])));
    assert_eq!(model.authority(), Some(&Authority::Unknown));
    assert!(model.target(next.visit, target()));
    assert!(!model.finish(opened.visit, Finish::Archive));
    assert_eq!(
        model.draft().unwrap().target().unwrap().finish(),
        Finish::Keep
    );
    assert!(model.finish(next.visit, Finish::Archive));
    assert_eq!(
        model.draft().unwrap().target().unwrap().finish(),
        Finish::Archive
    );
}

#[test]
fn a_preview_read_refusal_cannot_claim_that_this_device_lacks_operate() {
    let (mut model, opened) = active();
    let request = model.preview(opened.visit, at()).unwrap().unwrap();
    assert!(matches!(
        model.received(
            &request,
            Err(Failure::Node(vessel_proto::WireError::new(
                vessel_proto::ErrorCode::Forbidden,
                "private read refusal"
            )))
        ),
        Received::Changed
    ));
    assert_eq!(model.authority(), Some(&Authority::Unknown));
    assert!(!model.blocked());
    assert!(model.field(opened.visit, FieldChange::Name("Retain editing".into())));
    assert!(matches!(model.prepare_save(), Err(Issue::Preview)));
    assert!(model.preview(opened.visit, at()).unwrap().is_some());
}
