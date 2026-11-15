//! Choices are typed node data, not fabricated model names or viewer paths.
use super::*;
use vessel_proto::workspace::{ProjectInfo, ProjectState, SessionPlace};
use vessel_proto::{ChoiceValue, Harness, HarnessOptions, OptionChoice, SessionMode};
fn project() -> ProjectInfo {
    ProjectInfo {
        id: "00000000-0000-4000-8000-000000000011".parse().unwrap(),
        name: "Project".parse().unwrap(),
        root: "/project".into(),
        git: true,
        default_branch: None,
        state: ProjectState::Ready,
        created_at: at(),
        revision: 1,
    }
}
pub(super) fn targets() -> io::Targets {
    io::Targets {
        projects: vec![project()],
        agents: vec![],
        harnesses: vec![Harness {
            kind: "claude-code".parse().unwrap(),
            path: "/fake-agent".into(),
            version: None,
            structured: vessel_proto::StructuredSupport::Available,
            terminal: vessel_proto::TerminalSupport::Available,
        }],
        timezone: Some(TimeZoneName::utc()),
    }
}
pub(super) fn options() -> HarnessOptions {
    HarnessOptions { options: serde_json::from_str(r#"[
      {"id":"model","name":"Model","category":"model","kind":"select","current":"new-model","values":[{"id":"old-model","name":"Opened model"},{"id":"new-model","name":"Other model"}]},
      {"id":"custom","name":"Custom","category":"other","kind":"toggle","on":false},
      {"id":"future","name":"Future","category":"other","kind":"unknown"}
    ]"#).unwrap(), probed_at: at(), abilities: None }
}
fn choice(id: &str, value: &str) -> OptionChoice {
    OptionChoice {
        id: vessel_proto::OptionId::new(id),
        value: ChoiceValue::Select {
            value: vessel_proto::OptionValueId::new(value),
        },
    }
}
#[test]
fn project_before_agent_is_node_listed_and_missing_agent_cannot_be_invented() {
    let mut model = Model::default();
    let visit = model
        .create(cadence(), TimeZoneName::utc())
        .unwrap()
        .unwrap();
    let opened = model.select_host(visit, connection()).unwrap();
    assert!(matches!(
        model.received(&opened.request, Ok(io::Value::Targets(Box::new(targets())))),
        Received::Changed
    ));
    let unknown = "00000000-0000-4000-8000-000000000012".parse().unwrap();
    assert!(!model.project(opened.visit, unknown));
    assert!(model.project(opened.visit, project().id));
    assert_eq!(model.project_id(), Some(project().id));
    assert!(
        model
            .agent(opened.visit, "codex".parse().unwrap())
            .is_none()
    );
    let options_request = model
        .agent(opened.visit, "claude-code".parse().unwrap())
        .unwrap();
    assert!(matches!(
        options_request.operation(),
        io::Operation::Options(_)
    ));
    assert_eq!(
        model
            .draft()
            .unwrap()
            .target()
            .unwrap()
            .spec()
            .place
            .as_deref(),
        Some(&SessionPlace::Project {
            project: project().id,
            dir: None
        })
    );
    assert!(model.field(opened.visit, FieldChange::Prompt("Do the work".into())));
    assert!(matches!(
        model.received(&options_request, Ok(io::Value::Options(options()))),
        Received::Changed
    ));
    assert!(model.controls(crate::Lang::En).unwrap().model.is_some());
}
#[test]
fn choosing_agent_alone_does_not_silently_use_the_nodes_current_directory() {
    let mut model = Model::default();
    let visit = model
        .create(cadence(), TimeZoneName::utc())
        .unwrap()
        .unwrap();
    let opened = model.select_host(visit, connection()).unwrap();
    assert!(matches!(
        model.received(&opened.request, Ok(io::Value::Targets(Box::new(targets())))),
        Received::Changed
    ));
    assert!(
        model
            .agent(opened.visit, "claude-code".parse().unwrap())
            .is_some()
    );
    assert!(model.field(opened.visit, FieldChange::Prompt("Do the work".into())));
    assert!(matches!(model.prepare_save(), Err(Issue::Target)));
}
#[test]
fn an_unready_project_is_never_available_for_a_new_schedule() {
    let (mut model, opened) = active();
    let mut offered = targets();
    offered.projects[0].state = ProjectState::Missing;
    assert!(matches!(
        model.received(&opened.request, Ok(io::Value::Targets(Box::new(offered)))),
        Received::Changed
    ));
    assert!(!model.project(opened.visit, project().id));
    assert_eq!(model.project_id(), None);
}
#[test]
fn explicit_option_changes_preserve_other_choices_and_full_spec_metadata() {
    let (mut model, opened) = active();
    assert!(matches!(
        model.received(&opened.request, Ok(io::Value::Targets(Box::new(targets())))),
        Received::Changed
    ));
    let request = model
        .agent(opened.visit, "claude-code".parse().unwrap())
        .unwrap();
    assert!(matches!(
        model.received(&request, Ok(io::Value::Options(options()))),
        Received::Changed
    ));
    assert_eq!(
        model
            .controls(crate::Lang::En)
            .unwrap()
            .model
            .unwrap()
            .current
            .as_str(),
        "old-model"
    );
    assert!(!model.option(opened.visit, choice("model", "not-offered")));
    assert!(!model.option(opened.visit, choice("future", "not-offered")));
    assert!(!model.option(opened.visit, choice("custom", "wrong-kind")));
    let toggle = OptionChoice {
        id: vessel_proto::OptionId::new("custom"),
        value: ChoiceValue::Toggle { on: true },
    };
    assert!(model.option(opened.visit, toggle.clone()));
    assert!(model.option(opened.visit, choice("model", "new-model")));
    let target = model.draft().unwrap().target().unwrap();
    let mut expected = super::target().spec().clone();
    expected.mode = SessionMode::Structured {
        model: None,
        resume: None,
        options: vec![toggle, choice("model", "new-model")],
    };
    assert_eq!(target.spec(), &expected);
    assert_eq!(target.finish(), Finish::Keep);
    assert_eq!(
        model
            .controls(crate::Lang::Ko)
            .unwrap()
            .model
            .unwrap()
            .current
            .as_str(),
        "new-model"
    );
}
#[test]
fn recurring_prompt_cannot_mutate_projects_even_when_targets_were_loaded() {
    let mut model = Model::default();
    let opened = model
        .edit(info(), connection(), TimeZoneName::utc())
        .unwrap()
        .unwrap();
    assert!(matches!(
        model.received(&opened.request, Ok(io::Value::Targets(Box::new(targets())))),
        Received::Changed
    ));
    assert!(!model.project(opened.visit, project().id));
    assert!(
        model
            .agent(opened.visit, "claude-code".parse().unwrap())
            .is_none()
    );
    assert_eq!(model.project_id(), None);
}

#[test]
fn save_waits_for_agent_models_and_never_substitutes_a_missing_opened_model() {
    let (mut model, opened) = active();
    let preview = model.preview(opened.visit, at()).unwrap().unwrap();
    assert!(matches!(
        model.received(&preview, Ok(response(&preview))),
        Received::Changed
    ));
    assert!(matches!(model.prepare_save(), Err(Issue::Target)));
    assert!(matches!(
        model.received(&opened.request, Ok(io::Value::Targets(Box::new(targets())))),
        Received::Changed
    ));
    let request = model
        .agent(opened.visit, "claude-code".parse().unwrap())
        .unwrap();
    assert!(matches!(model.prepare_save(), Err(Issue::Target)));
    let mut offered = options();
    let vessel_proto::OptionKind::Select { values, .. } = &mut offered.options[0].kind else {
        panic!("model select")
    };
    values.retain(|value| value.id.as_str() != "old-model");
    assert!(matches!(
        model.received(&request, Ok(io::Value::Options(offered))),
        Received::Changed
    ));
    assert!(matches!(model.prepare_save(), Err(Issue::Target)));
    let displayed = model.controls(crate::Lang::En).unwrap().model.unwrap();
    assert_eq!(displayed.current.as_str(), "old-model");
    assert!(displayed.current_value().is_none());
    assert_eq!(
        model.draft().unwrap().target().unwrap().spec(),
        target().spec()
    );
    assert!(model.option(opened.visit, choice("model", "new-model")));
    assert!(model.prepare_save().unwrap().is_some());
}

fn two_providers() -> io::Targets {
    let mut available = targets();
    let mut second = available.harnesses.first().unwrap().clone();
    second.kind = "codex".parse().unwrap();
    available.harnesses.push(second);
    available
}
#[test]
fn browsing_another_provider_never_changes_the_selected_target_before_a_model_pick() {
    let (mut model, opened) = active();
    assert!(matches!(
        model.received(
            &opened.request,
            Ok(io::Value::Targets(Box::new(two_providers())))
        ),
        Received::Changed
    ));
    let selected = model
        .agent(opened.visit, "claude-code".parse().unwrap())
        .unwrap();
    assert!(matches!(
        model.received(&selected, Ok(io::Value::Options(options()))),
        Received::Changed
    ));
    let original = model.draft().unwrap().target().unwrap().spec().clone();
    let browsing = model
        .browse_agent(opened.visit, "codex".parse().unwrap())
        .unwrap();
    assert_eq!(model.draft().unwrap().target().unwrap().spec(), &original);
    assert_eq!(model.selected_options_request(), Some(&selected));
    assert!(matches!(
        model.received(&browsing, Ok(io::Value::Options(options()))),
        Received::Changed
    ));
    assert_eq!(model.draft().unwrap().target().unwrap().spec(), &original);
    assert_eq!(
        model
            .controls(crate::Lang::En)
            .unwrap()
            .model
            .unwrap()
            .current
            .as_str(),
        "old-model"
    );
    assert_eq!(
        model
            .catalog_controls(crate::Lang::En)
            .unwrap()
            .model
            .unwrap()
            .current
            .as_str(),
        "new-model"
    );
    assert!(!model.choose_model(opened.visit, &browsing, choice("model", "unoffered")));
    assert!(!model.choose_model(opened.visit, &selected, choice("model", "new-model")));
    assert_eq!(model.draft().unwrap().target().unwrap().spec(), &original);
    assert!(model.choose_model(opened.visit, &browsing, choice("model", "new-model")));
    let target = model.draft().unwrap().target().unwrap();
    let mut expected = original;
    expected.harness = "codex".parse().unwrap();
    expected.profile = None;
    expected.mode = SessionMode::Structured {
        model: None,
        resume: None,
        options: vec![choice("model", "new-model")],
    };
    assert_eq!(target.spec(), &expected);
    assert_eq!(target.finish(), Finish::Keep);
    assert_eq!(model.selected_options_request(), Some(&browsing));
    assert_eq!(model.initial_browser_harness().unwrap().as_str(), "codex");
}
#[test]
fn model_browser_opens_at_the_sole_provider_or_an_explicit_installed_selection() {
    let mut model = Model::default();
    let visit = model
        .create(cadence(), TimeZoneName::utc())
        .unwrap()
        .unwrap();
    let opened = model.select_host(visit, connection()).unwrap();
    assert!(model.initial_browser_harness().is_none());
    assert!(matches!(
        model.received(
            &opened.request,
            Ok(io::Value::Targets(Box::new(two_providers())))
        ),
        Received::Changed
    ));
    assert!(model.initial_browser_harness().is_none());
    let request = model
        .browse_agent(opened.visit, "codex".parse().unwrap())
        .unwrap();
    assert!(model.draft().unwrap().target().is_none());
    assert!(matches!(
        model.received(&request, Ok(io::Value::Options(options()))),
        Received::Changed
    ));
    assert!(model.initial_browser_harness().is_none());
    assert!(model.choose_model(opened.visit, &request, choice("model", "old-model")));
    assert_eq!(model.initial_browser_harness().unwrap().as_str(), "codex");
    let (mut sole, opened) = active();
    assert!(matches!(
        sole.received(&opened.request, Ok(io::Value::Targets(Box::new(targets())))),
        Received::Changed
    ));
    assert_eq!(
        sole.initial_browser_harness().unwrap().as_str(),
        "claude-code"
    );
}
