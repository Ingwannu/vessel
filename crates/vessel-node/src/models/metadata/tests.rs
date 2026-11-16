//! Real catalog parsing and typed preferences, with no provider or agent.

use super::*;
use std::fmt::Write as _;
use vessel_proto::settings::MetadataProvider;

fn catalog(entries: &[(&str, &str, Option<&str>)]) -> Routes {
    let mut text = String::from(
        r#"
[providers.fixture]
api = "anthropic-messages"
base_url = "https://fixture.invalid/v1"
key = { secret = "FIXTURE_KEY" }
"#,
    );
    for (name, model, small) in entries {
        writeln!(
            text,
            "[models.{name}]\nprovider = \"fixture\"\nmodel = {model:?}"
        )
        .unwrap();
        if let Some(small) = small {
            writeln!(text, "small_model = {small:?}").unwrap();
        }
    }
    super::super::config::parse(&text).unwrap()
}
fn candidate(route: &str, model: Option<&str>, effort: Option<MetadataEffort>) -> MetadataProvider {
    MetadataProvider {
        route: route.parse().unwrap(),
        model: model.map(ModelId::new),
        effort,
    }
}
fn manual(entries: Vec<MetadataProvider>) -> MetadataSettings {
    MetadataSettings::new(entries).unwrap()
}
fn assert_choice(choice: &Selection<'_>, route: &str, model: &str, effort: Option<MetadataEffort>) {
    assert_eq!(choice.route().name.as_str(), route);
    assert_eq!(choice.provider().name.as_str(), "fixture");
    assert_eq!(choice.model().as_str(), model);
    assert_eq!(choice.effort(), effort);
}

#[test]
fn each_hint_has_reference_priority_and_keeps_its_model() {
    let entries = [
        ("a-nemo", "nemotron-3-super", None),
        ("b-mini", "minimax-m3", None),
        ("c-gpt", "gpt-5.4-mini", None),
        ("d-fast", "HaIkU", Some("unrelated-small")),
    ];
    let settings = MetadataSettings::default();
    let all = catalog(&entries);
    assert_choice(
        &select(&all, &settings, None).unwrap(),
        "d-fast",
        "HaIkU",
        None,
    );
    let rest = catalog(&entries[..3]);
    assert_choice(
        &select(&rest, &settings, None).unwrap(),
        "c-gpt",
        "gpt-5.4-mini",
        Some(MetadataEffort::Low),
    );
    let rest = catalog(&entries[..2]);
    assert_choice(
        &select(&rest, &settings, None).unwrap(),
        "b-mini",
        "minimax-m3",
        None,
    );
    let rest = catalog(&entries[..1]);
    assert_choice(
        &select(&rest, &settings, None).unwrap(),
        "a-nemo",
        "nemotron-3-super",
        None,
    );
}

#[test]
fn ties_use_sorted_routes_and_matching_primary_precedes_small() {
    let routes = catalog(&[
        ("z-fast", "haiku-z", None),
        ("a-fast", "haiku-main", Some("haiku-small")),
    ]);
    let settings = MetadataSettings::default();
    assert_choice(
        &select(&routes, &settings, None).unwrap(),
        "a-fast",
        "haiku-main",
        None,
    );
}

#[test]
fn matching_small_and_name_only_alias_choose_the_intended_model() {
    let settings = MetadataSettings::default();
    let routes = catalog(&[("plain", "main", Some("haiku-small"))]);
    assert_choice(
        &select(&routes, &settings, None).unwrap(),
        "plain",
        "haiku-small",
        None,
    );
    let routes = catalog(&[("haiku-alias", "main", Some("small"))]);
    assert_choice(
        &select(&routes, &settings, None).unwrap(),
        "haiku-alias",
        "small",
        None,
    );
}

#[test]
fn current_route_is_only_the_automatic_fallback() {
    let settings = MetadataSettings::default();
    let name = "current".parse().unwrap();
    let routes = catalog(&[("current", "main", Some("small"))]);
    assert_choice(
        &select(&routes, &settings, Some(&name)).unwrap(),
        "current",
        "small",
        None,
    );
    assert!(matches!(
        select(&routes, &settings, None),
        Err(SelectError::NoRoute)
    ));
    let routes = catalog(&[("current", "main", None)]);
    assert_choice(
        &select(&routes, &settings, Some(&name)).unwrap(),
        "current",
        "main",
        None,
    );
}

#[test]
fn manual_order_is_isolated_and_missing_routes_do_not_switch_to_auto() {
    let routes = catalog(&[
        ("haiku-auto", "haiku", None),
        ("z-manual", "manual", Some("manual-small")),
    ]);
    let settings = manual(vec![
        candidate("z-manual", None, None),
        candidate("haiku-auto", None, None),
    ]);
    assert_choice(
        &select(&routes, &settings, None).unwrap(),
        "z-manual",
        "manual-small",
        None,
    );
    let settings = manual(vec![
        candidate("gone", None, None),
        candidate("z-manual", None, None),
    ]);
    assert_choice(
        &select(&routes, &settings, None).unwrap(),
        "z-manual",
        "manual-small",
        None,
    );
    let settings = manual(vec![candidate("gone", None, None)]);
    assert!(matches!(
        select(&routes, &settings, Some(&"haiku-auto".parse().unwrap())),
        Err(SelectError::NoRoute)
    ));
}

#[test]
fn explicit_models_and_duplicates_keep_the_first_tuple() {
    let routes = catalog(&[("chosen", "main", Some("small"))]);
    for target in ["main", "small"] {
        let entry = candidate("chosen", Some(target), Some(MetadataEffort::High));
        let settings = manual(vec![entry.clone(), entry]);
        assert_choice(
            &select(&routes, &settings, None).unwrap(),
            "chosen",
            target,
            Some(MetadataEffort::High),
        );
    }
}

#[test]
fn reloaded_model_or_future_effort_is_refused_without_falling_through() {
    let routes = catalog(&[("chosen", "new-model", None), ("next", "next-model", None)]);
    let settings = manual(vec![
        candidate("chosen", Some("old-model"), None),
        candidate("next", None, None),
    ]);
    assert!(
        matches!(select(&routes, &settings, None), Err(SelectError::ModelNotAllowed(route)) if route.as_str()=="chosen")
    );
    let settings = manual(vec![
        candidate("chosen", None, Some(MetadataEffort::Unknown)),
        candidate("next", None, None),
    ]);
    assert!(matches!(
        select(&routes, &settings, None),
        Err(SelectError::UnknownEffort)
    ));
    assert!(matches!(
        select(&Routes::default(), &MetadataSettings::default(), None),
        Err(SelectError::NoRoute)
    ));
}

#[test]
fn selection_debug_does_not_expose_provider_configuration() {
    let routes = catalog(&[("haiku-fast", "haiku", None)]);
    let settings = MetadataSettings::default();
    let selected = select(&routes, &settings, None).unwrap();
    let debug = format!("{selected:?}");
    assert!(debug.contains("haiku-fast"));
    assert!(!debug.contains("fixture.invalid"));
    assert!(!debug.contains("FIXTURE_KEY"));
}
