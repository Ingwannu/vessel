//! Exercise the mutation edge; the underlying key/value rules stay in labels' tests.

use super::*;

#[test]
fn an_empty_patch_is_refused_by_construction_and_deserialization() {
    assert_eq!(
        LabelPatch::try_from(SessionLabels::new()),
        Err(InvalidLabel::Empty)
    );
    let error = serde_json::from_str::<LabelPatch>("{}").unwrap_err();
    assert!(error.is_data());
    assert!(error.to_string().contains(&InvalidLabel::Empty.to_string()));
}

#[test]
fn a_patch_retains_the_map_shape_and_last_valid_duplicate_without_trimming_values() {
    let text = r#"{"env":"old"," env ":" phase = one, two "}"#;
    let patch: LabelPatch = serde_json::from_str(text).unwrap();
    let mut expected = SessionLabels::new();
    expected
        .insert(
            "env".parse().unwrap(),
            " phase = one, two ".parse().unwrap(),
        )
        .unwrap();
    assert_eq!(patch.labels(), &expected);
    let encoded = serde_json::to_string(&patch).unwrap();
    assert_eq!(encoded, serde_json::to_string(&expected).unwrap());
    assert_eq!(serde_json::from_str::<LabelPatch>(&encoded).unwrap(), patch);
}

#[test]
fn an_empty_value_still_names_a_change() {
    let patch: LabelPatch = serde_json::from_str(r#"{"env":""}"#).unwrap();
    let mut expected = SessionLabels::new();
    expected
        .insert("env".parse().unwrap(), "".parse().unwrap())
        .unwrap();
    assert_eq!(patch.labels(), &expected);
}

#[test]
fn the_patch_cannot_bypass_the_existing_map_decoder() {
    for text in [
        "null",
        "[]",
        r#"{"env":null}"#,
        r#"{"env":true}"#,
        r#"{"":"value"}"#,
        r#"{"vessel.owner":"value"}"#,
        r#"{"env":"\u0085"}"#,
    ] {
        assert!(serde_json::from_str::<LabelPatch>(text).is_err(), "{text}");
    }
}

#[test]
fn bounded_decoding_keeps_the_maps_single_cap() {
    let mut full = SessionLabels::new();
    for index in 0..SessionLabels::MAX {
        full.insert(
            format!("key-{index}").parse().unwrap(),
            "v".parse().unwrap(),
        )
        .unwrap();
    }
    let text = serde_json::to_string(&full).unwrap();
    let patch: LabelPatch = serde_json::from_str(&text).unwrap();
    assert_eq!(patch.labels(), &full);
    let pairs: Vec<String> = (0..SessionLabels::MAX.saturating_add(1))
        .map(|index| format!(r#""key-{index}":"v""#))
        .collect();
    let oversized = format!("{{{}}}", pairs.join(","));
    assert!(serde_json::from_str::<LabelPatch>(&oversized).is_err());
}
