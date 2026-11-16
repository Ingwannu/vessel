//! Cache preparation is independent from widgets and never fabricates node times.
use super::*;
use crate::localization::{ENGLISH, Locale};
use crate::schedules::{
    authority::Authority,
    editor::session::FieldChange,
    page::tests::{context, key},
};
use vessel_proto::automation::{Cadence, SchedulePreview, TimeZoneName};
fn catalogs() -> [(&'static Translator, &'static str); 2] {
    [
        (Translator::get(ENGLISH).unwrap(), "New schedule"),
        (
            Translator::get(Locale::from_tag("ko").unwrap()).unwrap(),
            "새 예약",
        ),
    ]
}
#[test]
fn new_form_words_and_feedback_are_cached_without_pristine_errors() {
    let mut model = Model::default();
    let zone = "Asia/Seoul".parse().unwrap();
    let cadence = super::super::cadence::initial(zone).unwrap();
    let visit = model.create(cadence, TimeZoneName::utc()).unwrap().unwrap();
    for (catalog, title) in catalogs() {
        let form = prepare(&model, &crate::Fleet::default(), &Clock::unknown(), catalog)
            .unwrap()
            .unwrap();
        assert_eq!(&*form.title, title);
        assert_eq!(
            form.text.iter().map(|text| text.field).collect::<Vec<_>>(),
            [Field::Name, Field::Prompt, Field::Limit]
        );
        assert!(form.text.iter().all(|text| text.error.is_none()));
        assert_eq!(form.submission, Submission::Unavailable);
        assert!(!form.viewer_known);
        assert!(form.next_runs.is_empty());
    }
    assert!(model.field(visit, FieldChange::Limit("1000001".into())));
    for (catalog, _) in catalogs() {
        let form = prepare(&model, &crate::Fleet::default(), &Clock::unknown(), catalog)
            .unwrap()
            .unwrap();
        assert!(form.text[1].error.is_none());
        assert!(form.text[2].error.is_some());
        assert_eq!(form.submission, Submission::Unavailable);
    }
}
#[test]
fn heartbeat_has_only_its_rule_and_only_current_node_preview_times() {
    let mut model = Model::default();
    let mut info = crate::schedules::tests::info();
    info.definition.cadence = super::super::cadence::initial(TimeZoneName::utc()).unwrap();
    let opened = model
        .edit(info, context(&key("laptop")), TimeZoneName::utc())
        .unwrap()
        .unwrap();
    let after = "2026-11-15T00:00:00Z".parse().unwrap();
    let request = model.preview(opened.visit, after).unwrap().unwrap();
    let io::Operation::Preview(input) = request.operation() else {
        panic!("preview")
    };
    assert!(matches!(input.cadence, Cadence::Cron { .. }));
    assert!(matches!(
        model.received(
            &request,
            Ok(io::Value::Preview(SchedulePreview {
                cadence: input.cadence.clone(),
                after,
                runs: [
                    "2026-11-15T01:00:00Z",
                    "2026-11-15T02:00:00Z",
                    "2026-11-15T03:00:00Z"
                ]
                .map(|at| at.parse().unwrap())
                .to_vec(),
            }))
        ),
        super::super::session::Received::Changed
    ));
    let clock = Clock::known("Asia/Seoul".parse().unwrap()).unwrap();
    for (catalog, _) in catalogs() {
        let form = prepare(&model, &crate::Fleet::default(), &clock, catalog)
            .unwrap()
            .unwrap();
        assert!(form.text.is_empty());
        assert_eq!(&*form.title, catalog.schedules().form_heartbeat_title());
        assert_eq!(form.next_runs.len(), 3);
        assert!(form.viewer_known);
        assert_eq!(form.submission, Submission::Ready);
    }
    assert!(model.cadence(opened.visit, |draft| {
        draft.preset(super::super::cadence::Preset::Daily)
    }));
    for (catalog, _) in catalogs() {
        let form = prepare(&model, &crate::Fleet::default(), &clock, catalog)
            .unwrap()
            .unwrap();
        assert!(form.next_runs.is_empty());
        assert_eq!(form.submission, Submission::Unavailable);
    }
    assert!(model.reported_authority(
        opened.visit,
        &Authority::Scopes(vec![vessel_proto::Scope::Read])
    ));
    for (catalog, _) in catalogs() {
        let form = prepare(&model, &crate::Fleet::default(), &clock, catalog)
            .unwrap()
            .unwrap();
        assert!(form.read_only.is_some());
        assert!(form.blocked);
    }
}

#[test]
fn completed_edit_keeps_preview_admission_without_claiming_next_execution() {
    use vessel_proto::automation::{EndReason, ScheduleState};
    let mut model = Model::default();
    let mut info = crate::schedules::tests::info();
    info.definition.cadence = super::super::cadence::initial(TimeZoneName::utc()).unwrap();
    info.state = ScheduleState::Ended {
        at: info.updated_at,
        reason: EndReason::MaxRuns,
    };
    let opened = model
        .edit(info, context(&key("laptop")), TimeZoneName::utc())
        .unwrap()
        .unwrap();
    let after = "2026-11-15T00:00:00Z".parse().unwrap();
    let request = model.preview(opened.visit, after).unwrap().unwrap();
    let io::Operation::Preview(input) = request.operation() else {
        panic!("preview")
    };
    assert!(matches!(
        model.received(
            &request,
            Ok(io::Value::Preview(SchedulePreview {
                cadence: input.cadence.clone(),
                after,
                runs: [
                    "2026-11-15T01:00:00Z",
                    "2026-11-15T02:00:00Z",
                    "2026-11-15T03:00:00Z"
                ]
                .map(|at| at.parse().unwrap())
                .to_vec(),
            }))
        ),
        super::super::session::Received::Changed
    ));
    for (catalog, _) in catalogs() {
        let form = prepare(&model, &crate::Fleet::default(), &Clock::unknown(), catalog)
            .unwrap()
            .unwrap();
        assert!(form.next_runs.is_empty());
        assert_eq!(form.submission, Submission::Ready);
    }
}
