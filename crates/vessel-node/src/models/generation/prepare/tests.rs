//! Real catalog/encoder rules without an HTTP client or credential reader.

use super::*;
use crate::models::config;
use vessel_proto::{
    ModelId,
    settings::{MetadataEffort, MetadataProvider},
};
use vessel_workspace::project_file::metadata::ProjectStyles;

fn catalog() -> Routes {
    config::parse(
        r#"
[providers.adaptive]
api = "anthropic-messages"
base_url = "https://not-called.invalid/v1"
adaptive_thinking = true
[providers.budget]
api = "anthropic-messages"
base_url = "https://not-called.invalid/v1"
[models.fast]
provider = "adaptive"
model = "fixture-main"
small_model = "fixture-small"
[models.budgeted]
provider = "budget"
model = "fixture-budget"
"#,
    )
    .unwrap()
}
fn choice(route: &str, model: Option<&str>, effort: Option<MetadataEffort>) -> MetadataProvider {
    MetadataProvider {
        route: route.parse().unwrap(),
        model: model.map(ModelId::new),
        effort,
    }
}
fn settings(entries: Vec<MetadataProvider>) -> MetadataSettings {
    MetadataSettings::new(entries).unwrap()
}

#[test]
fn automatic_defaults_need_no_route_but_every_manual_entry_must_resolve() {
    validate(&Routes::default(), &MetadataSettings::default()).unwrap();
    let catalog = catalog();
    let manual = settings(vec![
        choice("fast", None, None),
        choice("missing", None, None),
    ]);
    assert!(matches!(
        validate(&catalog, &manual),
        Err(PrepareError::Candidate(SelectError::MissingRoute(_)))
    ));
}

#[test]
fn a_later_model_override_or_future_effort_cannot_be_saved() {
    let catalog = catalog();
    let wrong = settings(vec![
        choice("fast", None, None),
        choice("fast", Some("outside-route"), None),
    ]);
    assert!(matches!(
        validate(&catalog, &wrong),
        Err(PrepareError::Candidate(SelectError::ModelNotAllowed(_)))
    ));
    let future = settings(vec![
        choice("fast", None, None),
        choice("fast", None, Some(MetadataEffort::Unknown)),
    ]);
    assert!(matches!(
        validate(&catalog, &future),
        Err(PrepareError::Candidate(SelectError::UnknownEffort))
    ));
    validate(
        &catalog,
        &settings(vec![
            choice("fast", Some("fixture-main"), None),
            choice("fast", Some("fixture-small"), None),
        ]),
    )
    .unwrap();
}

#[test]
fn actual_thinking_budget_is_checked_even_on_a_later_candidate() {
    let catalog = catalog();
    let over_budget = settings(vec![
        choice("fast", None, None),
        choice("budgeted", None, Some(MetadataEffort::Low)),
    ]);
    let error = validate(&catalog, &over_budget).unwrap_err();
    assert!(
        matches!(error, PrepareError::Request(RequestError::Budget { limit }) if limit.get() == TITLE_OUTPUT_TOKENS)
    );
    validate(
        &catalog,
        &settings(vec![
            choice("budgeted", None, None),
            choice("fast", None, Some(MetadataEffort::Low)),
        ]),
    )
    .unwrap();
}

#[test]
fn actual_preparation_uses_checked_model_source_style_and_output_budget() {
    let catalog = catalog();
    let settings = settings(vec![choice(
        "fast",
        Some("fixture-main"),
        Some(MetadataEffort::Low),
    )]);
    let selected = metadata::select(&catalog, &settings, None).unwrap();
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(
        folder.path().join("vessel.toml"),
        "[metadata.title]\ninstructions = 'Use the replacement fixture style.'\n",
    )
    .unwrap();
    let styles = ProjectStyles::read(folder.path());
    let material = Material::capture("private source material marker").unwrap();
    let prepared = title(&selected, &material, styles.title.as_ref()).unwrap();
    assert_eq!(prepared.api(), Api::AnthropicMessages);
    assert!(!format!("{prepared:?}").contains("private source material marker"));
    let bytes = prepared.into_body();
    let body: vessel_llm::wire::anthropic::Request = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.model, "fixture-main");
    assert_eq!(body.max_tokens, Some(TITLE_OUTPUT_TOKENS));
    assert_eq!(body.stream, Some(false));
    let encoded = std::str::from_utf8(&bytes).unwrap();
    assert!(encoded.contains("private source material marker"));
    assert!(encoded.contains("replacement fixture style"));
    assert_eq!(output_limit().unwrap().get(), TITLE_OUTPUT_TOKENS);
}

#[test]
fn an_omitted_model_uses_the_same_small_choice_in_preflight_and_generation() {
    let catalog = catalog();
    let settings = settings(vec![choice("fast", None, None)]);
    validate(&catalog, &settings).unwrap();
    let selected = metadata::select(&catalog, &settings, None).unwrap();
    let prepared = title(
        &selected,
        &Material::capture("fixture source").unwrap(),
        None,
    )
    .unwrap();
    let body: vessel_llm::wire::anthropic::Request =
        serde_json::from_slice(&prepared.into_body()).unwrap();
    assert_eq!(body.model, "fixture-small");
    assert_eq!(body.max_tokens, Some(TITLE_OUTPUT_TOKENS));
}
