use super::*;
use crate::schedules::{editor::cadence::Preset, tests::info};
use vessel_proto::automation::ScheduleState;

fn target() -> NewTarget {
    NewTarget::new(
        serde_json::from_str(
            r#"{
        "harness":"claude-code","mode":"structured","model":"model-id",
        "profile":"work","title":"Original title","cwd":"/a/long/한글 path",
        "labels":{"project":"keep-me"},
        "side_question_of":"00000000-0000-4000-8000-000000000099"
    }"#,
        )
        .unwrap(),
        Finish::default(),
    )
    .unwrap()
}
#[test]
fn a_recurring_prompt_exposes_only_cadence_and_keeps_its_original_revision() {
    let original = info();
    let mut draft = Draft::open(original.clone(), TimeZoneName::utc()).unwrap();
    assert!(draft.fields().is_none());
    assert!(draft.fields_mut().is_none());
    assert!(draft.target_mut().is_none());
    draft.cadence.preset(Preset::Weekdays);
    let Save::Update(saved) = draft.save().unwrap() else {
        panic!("not an update")
    };
    assert_eq!(saved.schedule, original.id);
    assert_eq!(saved.revision, original.revision);
    let mut expected = original.definition;
    expected.cadence = draft.cadence.checked().unwrap().clone();
    assert_eq!(saved.definition, expected);
}
#[test]
fn a_new_chat_edit_keeps_the_complete_spec_and_node_only_definition_fields() {
    let mut original = info();
    original.definition.target = target().wire();
    original.definition.catch_up = CatchUp::Skip;
    original.definition.ends_at = Some("2027-01-01T00:00:00Z".parse().unwrap());
    let mut draft = Draft::open(original.clone(), TimeZoneName::utc()).unwrap();
    draft.fields_mut().unwrap().set_name("Renamed".into());
    let Save::Update(saved) = draft.save().unwrap() else {
        panic!("not an update")
    };
    let mut expected = original.definition.clone();
    expected.name = Some("Renamed".parse().unwrap());
    assert_eq!(saved.definition, expected);
    assert_eq!(saved.revision, original.revision);
    draft
        .target_mut()
        .unwrap()
        .as_mut()
        .unwrap()
        .set_finish(Finish::Keep);
    let Save::Update(saved) = draft.save().unwrap() else {
        panic!("not an update")
    };
    let ScheduleTarget::NewSession { spec, after } = saved.definition.target else {
        panic!("not a new session")
    };
    assert_eq!(spec.as_ref(), target().spec());
    assert_eq!(after, AfterRun::Keep);
}
#[test]
fn create_requires_valid_fields_and_target_and_uses_archive_by_default() {
    let mut draft = Draft::create(info().definition.cadence, TimeZoneName::utc()).unwrap();
    assert!(matches!(draft.save(), Err(Issue::Field(_))));
    draft.fields_mut().unwrap().set_prompt(" New work ".into());
    assert!(matches!(draft.save(), Err(Issue::Target)));
    *draft.target_mut().unwrap() = Some(target());
    let Save::Create(saved) = draft.save().unwrap() else {
        panic!("not a create")
    };
    assert!(!saved.paused);
    assert_eq!(saved.definition.prompt.as_str(), "New work");
    assert_eq!(saved.definition.catch_up, CatchUp::Once);
    assert!(saved.definition.ends_at.is_none());
    assert_eq!(saved.definition.target, target().wire());
}
#[test]
fn ended_unknown_or_unsupported_forms_never_become_editable() {
    let mut original = info();
    original.state = ScheduleState::Unknown;
    assert!(matches!(
        Draft::open(original, TimeZoneName::utc()),
        Err(EditError::Unknown)
    ));
    let mut ended = info();
    ended.state = ScheduleState::Ended {
        at: ended.updated_at,
        reason: vessel_proto::automation::EndReason::MaxRuns,
    };
    assert!(matches!(
        Draft::open(ended, TimeZoneName::utc()),
        Err(EditError::Ended)
    ));
    let spec = serde_json::from_str(r#"{"harness":"claude-code","mode":"terminal"}"#).unwrap();
    assert!(matches!(
        NewTarget::new(spec, Finish::Keep),
        Err(EditError::SessionMode)
    ));
    let spec = serde_json::from_str(
        r#"{"harness":"claude-code","mode":"structured","resume":"old-session"}"#,
    )
    .unwrap();
    assert!(matches!(
        NewTarget::new(spec, Finish::Keep),
        Err(EditError::SessionMode)
    ));
}

#[test]
fn borrowed_issues_delegate_existing_tables_and_new_target_copy_in_both_languages() {
    use crate::localization::{ENGLISH, Locale, Translator};
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    assert_eq!(
        Issue::Target.label(en),
        "Choose an agent and a folder for this schedule."
    );
    assert_eq!(
        Issue::Target.label(ko),
        "예약에 사용할 에이전트와 폴더를 선택하세요."
    );
    assert_eq!(
        Issue::Preview.label(en),
        "Preview the current rule before saving."
    );
    assert_eq!(
        Issue::Preview.label(ko),
        "저장하기 전에 현재 규칙의 실행 시각을 미리 확인하세요."
    );
    let mut draft = Draft::create(info().definition.cadence, TimeZoneName::utc()).unwrap();
    draft.cadence.cron("bad".into());
    let cadence = draft.cadence.checked().unwrap_err();
    let field = Fields::empty();
    let field_error = field.error().unwrap();
    for (catalog, lang) in [(en, crate::Lang::En), (ko, crate::Lang::Ko)] {
        assert_eq!(
            Issue::Cadence(cadence).label(catalog),
            lang.cadence_error(cadence)
        );
        assert_eq!(
            Issue::Field(field_error).label(catalog),
            field_error.label(catalog)
        );
        for error in [EditError::Unknown, EditError::Ended, EditError::SessionMode] {
            assert_eq!(Issue::Edit(error).label(catalog), lang.edit_error(error));
        }
    }
}
