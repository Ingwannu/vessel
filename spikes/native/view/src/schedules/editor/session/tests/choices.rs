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
fn targets() -> io::Targets {
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
fn options() -> HarnessOptions {
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
