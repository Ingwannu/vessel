//! The retained view is lenient; a wire patch remains strict at both levels.

use super::*;
use serde_json::json;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
struct View(#[serde(deserialize_with = "MetadataSettings::deserialize_view")] MetadataSettings);

#[test]
fn retained_views_ignore_additions_and_keep_every_known_value() {
    for raw in [
        r#"{"future_metadata":{"keep":"aside"},"providers":[{"route":"fast","model":"fixture-model","effort":"low"}]}"#,
        r#"{"providers":[{"route":"fast","model":"fixture-model","effort":"low","future_candidate":{"keep":"aside"}}]}"#,
    ] {
        let view: View = serde_json::from_str(raw).unwrap();
        let provider = &view.0.providers()[0];
        assert_eq!(provider.route.as_str(), "fast");
        assert_eq!(provider.model.as_ref().unwrap().as_str(), "fixture-model");
        assert_eq!(provider.effort, Some(MetadataEffort::Low));
        assert!(serde_json::from_str::<MetadataSettings>(raw).is_err());
    }
    let absent: View = serde_json::from_str(r#"{"future_metadata":true}"#).unwrap();
    assert!(absent.0.is_automatic());
}

#[test]
fn future_effort_is_readable_but_known_bad_shapes_still_fail() {
    let later: View =
        serde_json::from_str(r#"{"providers":[{"route":"fast","effort":"later"}]}"#).unwrap();
    assert_eq!(later.0.providers()[0].effort, Some(MetadataEffort::Unknown));
    for raw in [
        r#"{"providers":null}"#,
        r#"{"providers":false}"#,
        r#"{"providers":[{"route":""}]}"#,
        r#"{"providers":[{"route":"fast","model":false}]}"#,
        r#"{"providers":[{"route":"fast","effort":false}]}"#,
        r#"{"providers":[{"model":"missing-route"}]}"#,
    ] {
        assert!(serde_json::from_str::<View>(raw).is_err());
    }
}

#[test]
fn the_same_streaming_bound_refuses_before_an_excess_file_provider_decodes() {
    let mut providers: Vec<_> = (0..MAX_METADATA_PROVIDERS)
        .map(|_| json!({"route":"fast","future_candidate":true}))
        .collect();
    let exact = json!({"providers":providers});
    let view: View = serde_json::from_value(exact).unwrap();
    assert_eq!(view.0.providers().len(), MAX_METADATA_PROVIDERS);
    providers.push(json!("this must not be decoded as a provider"));
    let error = serde_json::from_value::<View>(json!({"providers":providers})).unwrap_err();
    assert!(error.to_string().contains(&InvalidMetadata.to_string()));
    assert!(!error.to_string().contains("expected struct"));
}
