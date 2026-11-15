//! Existing preferences seed only a current fresh selection; metadata stays intact.
use super::*;
use vessel_proto::{ChoiceValue, OptionChoice, SessionMode};
fn pick(value: &str) -> OptionChoice {
    OptionChoice {
        id: "model".parse().unwrap(),
        value: ChoiceValue::Select {
            value: value.parse().unwrap(),
        },
    }
}
fn fresh(options: vessel_proto::HarnessOptions) -> (Model, Opened) {
    let mut model = Model::default();
    let visit = model
        .create(cadence(), TimeZoneName::utc())
        .unwrap()
        .unwrap();
    let opened = model.select_host(visit, connection()).unwrap();
    let targets = choices::targets();
    let project = targets.projects.first().unwrap().id;
    assert!(matches!(
        model.received(&opened.request, Ok(io::Value::Targets(Box::new(targets)))),
        Received::Changed
    ));
    assert!(model.project(opened.visit, project));
    // An unavailable remembered provider falls back only to an actual available one.
    let preferred = "codex".parse().unwrap();
    let harness = model.initial_agent(Some(&preferred)).cloned().unwrap();
    assert_eq!(harness, "claude-code".parse().unwrap());
    let request = model.agent(opened.visit, harness).unwrap();
    assert!(matches!(
        model.received(&request, Ok(io::Value::Options(options))),
        Received::Changed
    ));
    assert!(model.field(opened.visit, FieldChange::Prompt("Check the build".into())));
    let preview = model.preview(opened.visit, at()).unwrap().unwrap();
    assert!(matches!(
        model.received(&preview, Err(Failure::Older)),
        Received::Changed
    ));
    (model, opened)
}
#[test]
fn fresh_choices_restore_only_offered_selection_categories_and_never_override_a_pick() {
    let (mut model, opened) = fresh(choices::options());
    assert_eq!(model.remembered_choices(), Some(vec![pick("new-model")]));
    let unrelated = OptionChoice {
        id: "custom".parse().unwrap(),
        value: ChoiceValue::Toggle { on: true },
    };
    assert!(model.restore_choices(opened.visit, &[pick("old-model"), unrelated]));
    assert_eq!(model.remembered_choices(), Some(vec![pick("old-model")]));
    assert!(!model.restore_choices(opened.visit, &[pick("new-model")]));
    assert_eq!(model.remembered_choices(), Some(vec![pick("old-model")]));
    let Save::Create(save) = model.prepare_save().unwrap().unwrap() else {
        panic!("not create");
    };
    let vessel_proto::automation::ScheduleTarget::NewSession { spec, .. } = save.definition.target
    else {
        panic!("not a new session");
    };
    assert!(
        matches!(spec.mode, SessionMode::Structured { model: None, ref options, .. } if options.as_slice() == [pick("old-model")].as_slice())
    );
    assert!(model.close(opened.visit));
    let _next = model
        .create(cadence(), TimeZoneName::utc())
        .unwrap()
        .unwrap();
    assert!(!model.restore_choices(opened.visit, &[pick("new-model")]));
}
#[test]
fn an_unoffered_default_cannot_become_a_save_ready_model() {
    let mut options = choices::options();
    let option = options.options.first_mut().unwrap();
    if let vessel_proto::OptionKind::Select { current, .. } = &mut option.kind {
        *current = "not-offered".parse().unwrap();
    }
    let (mut model, opened) = fresh(options);
    assert!(matches!(model.prepare_save(), Err(Issue::Target)));
    assert_eq!(model.remembered_choices(), Some(vec![]));
    assert!(!model.restore_choices(opened.visit, &[pick("not-offered")]));
    assert!(matches!(model.prepare_save(), Err(Issue::Target)));
    assert!(model.restore_choices(opened.visit, &[pick("old-model")]));
    assert!(matches!(model.prepare_save(), Ok(Some(_))));
}
#[test]
fn explicit_stored_option_wins_legacy_display_without_rewriting_the_opened_spec() {
    let mut opened_info = info();
    let mut spec = target().spec().clone();
    let SessionMode::Structured { options, .. } = &mut spec.mode else {
        panic!("structured");
    };
    options.push(pick("new-model"));
    opened_info.definition.target = vessel_proto::automation::ScheduleTarget::NewSession {
        spec: Box::new(spec),
        after: vessel_proto::automation::AfterRun::Keep,
    };
    let original = opened_info.definition.clone();
    let mut model = Model::default();
    let opened = model
        .edit(opened_info, connection(), TimeZoneName::utc())
        .unwrap()
        .unwrap();
    load_options(&mut model, opened.visit);
    assert_eq!(
        model
            .controls(crate::Lang::En)
            .unwrap()
            .model
            .unwrap()
            .current
            .as_str(),
        "new-model"
    );
    assert_eq!(model.remembered_choices(), Some(vec![pick("new-model")]));
    assert!(!model.restore_choices(opened.visit, &[pick("old-model")]));
    let preview = model.preview(opened.visit, at()).unwrap().unwrap();
    assert!(matches!(
        model.received(&preview, Err(Failure::Older)),
        Received::Changed
    ));
    let Save::Update(save) = model.prepare_save().unwrap().unwrap() else {
        panic!("not update");
    };
    assert_eq!(save.definition, original);
    assert_eq!(save.revision, info().revision);
}

#[test]
fn preferences_use_only_first_named_slots_and_preserve_the_offered_order() {
    let mut options = choices::options();
    let template = options.options.first().unwrap().clone();
    let mut effort = template.clone();
    effort.id = "reasoning".parse().unwrap();
    effort.category = vessel_proto::OptionCategory::ThoughtLevel;
    let mut duplicate = template.clone();
    duplicate.id = "second-model".parse().unwrap();
    let mut mode = template;
    mode.id = "permission-mode".parse().unwrap();
    mode.category = vessel_proto::OptionCategory::Mode;
    options.options.insert(0, effort);
    options.options.push(duplicate);
    options.options.push(mode);
    let (mut model, opened) = fresh(options);
    let named = |id: &str| OptionChoice {
        id: id.parse().unwrap(),
        value: ChoiceValue::Select {
            value: "new-model".parse().unwrap(),
        },
    };
    assert_eq!(
        model.remembered_choices(),
        Some(vec![
            named("reasoning"),
            named("model"),
            named("permission-mode")
        ])
    );
    let duplicate = OptionChoice {
        id: "second-model".parse().unwrap(),
        value: ChoiceValue::Select {
            value: "old-model".parse().unwrap(),
        },
    };
    assert!(!model.restore_choices(opened.visit, &[duplicate]));
    assert_eq!(
        model.remembered_choices(),
        Some(vec![
            named("reasoning"),
            named("model"),
            named("permission-mode")
        ])
    );
}
