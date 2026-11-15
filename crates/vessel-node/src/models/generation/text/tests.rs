//! Synthetic response objects exist only at this decoding edge's tests.

use super::*;
use serde_json::json;
use std::{error::Error, fmt::Write as _};
use vessel_llm::{
    client::{Api, ApiKey},
    completion,
};
use vessel_workspace::project_file::metadata::ProjectStyles;

fn completion(text: &str) -> CompletionText {
    let body = json!({
        "id":"msg_fixture", "type":"message", "role":"assistant", "model":"fixture-model",
        "content":[{"type":"text", "text":text}], "stop_reason":"end_turn", "stop_sequence":null,
        "usage":{"input_tokens":1,"output_tokens":1}
    });
    completion::decode(
        Api::AnthropicMessages,
        &serde_json::to_vec(&body).unwrap(),
        &Redactor::default(),
    )
    .text
    .unwrap()
}

#[test]
fn raw_source_cap_counts_unicode_and_remembers_a_cut() {
    let exact = "😀".repeat(MAX_SOURCE_CHARS);
    let whole = Material::capture(&exact).unwrap();
    assert_eq!(whole.text.chars().count(), MAX_SOURCE_CHARS);
    assert_eq!(whole.clipping, Clipping::Whole);
    let cut = Material::capture(&format!("{exact}hidden-tail")).unwrap();
    assert_eq!(cut.text, whole.text);
    assert_eq!(cut.clipping, Clipping::Cut);
    let prompt = request(&ModelId::new("fixture-model"), None, &cut, None).unwrap();
    assert_eq!(format!("{prompt:?}"), "Prompt(..)");
    let request = prompt.into_request();
    let ir::Part::Text(text) = &request.messages[0].parts[0] else {
        panic!("expected text");
    };
    assert!(!text.contains("hidden-tail"));
    assert!(text.contains("was cut"));
    assert_eq!(format!("{cut:?}"), "Material(..)");
    assert!(Material::capture(&" ".repeat(MAX_SOURCE_CHARS)).is_none());
    assert!(Material::capture(&format!("{}later text", " ".repeat(MAX_SOURCE_CHARS))).is_none());
}

#[test]
fn style_replaces_only_the_style_and_cannot_remove_the_contract() {
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(
        folder.path().join("vessel.toml"),
        "[metadata.title]\ninstructions = 'Use a fixture-specific style.'\n",
    )
    .unwrap();
    let styles = ProjectStyles::read(folder.path());
    let material = Material::capture("private source marker").unwrap();
    let model = ModelId::new("fixture-model");
    let plain = request(&model, None, &material, None)
        .unwrap()
        .into_request();
    let custom = request(&model, None, &material, styles.title.as_ref())
        .unwrap()
        .into_request();
    assert!(plain.system[0].contains("short, specific"));
    assert!(custom.system[0].contains("fixture-specific"));
    assert!(!custom.system[0].contains("short, specific"));
    for request in [&plain, &custom] {
        assert!(request.system[0].contains(&MAX_GENERATED_TITLE_UNITS.to_string()));
        assert!(request.system[0].contains("Return only one JSON object"));
        assert!(request.system[0].contains("untrusted data"));
        assert!(request.tools.is_empty());
        assert_eq!(request.tool_choice, ir::ToolChoice::None);
    }
    assert_eq!(plain.messages, custom.messages);
    assert!(matches!(
        request(&model, Some(MetadataEffort::Unknown), &material, None),
        Err(TextError::UnknownEffort)
    ));
}

#[test]
fn every_known_effort_is_carried_without_a_default_guess() {
    for (input, output) in [
        (MetadataEffort::Minimal, ir::Effort::Minimal),
        (MetadataEffort::Low, ir::Effort::Low),
        (MetadataEffort::Medium, ir::Effort::Medium),
        (MetadataEffort::High, ir::Effort::High),
        (MetadataEffort::Max, ir::Effort::Max),
    ] {
        assert_eq!(
            reasoning(Some(input)).unwrap(),
            ir::Reasoning::Effort(output)
        );
    }
    assert_eq!(reasoning(None).unwrap(), ir::Reasoning::Default);
}

#[test]
fn titles_are_checked_before_trim_and_blank_text_has_no_title() {
    let redactor = Redactor::default();
    let good = decode(
        &completion(r#"{"title":"  Repair fixtures  ","later_key":true}"#),
        &redactor,
    )
    .unwrap();
    assert_eq!(format!("{good:?}"), "Candidate(..)");
    assert_eq!(good.into_title().unwrap().to_string(), "Repair fixtures");
    assert!(
        decode(&completion(r#"{"title":"   "}"#), &redactor)
            .unwrap()
            .into_title()
            .is_none()
    );
    for text in [
        "",
        "two\nlines",
        "tab\there",
        &"a".repeat(MAX_GENERATED_TITLE_UNITS.saturating_add(1)),
    ] {
        let text = serde_json::to_string(&json!({"title":text})).unwrap();
        assert!(matches!(
            decode(&completion(&text), &redactor),
            Err(TextError::Title(_))
        ));
    }
    let padded = format!(" {} ", "a".repeat(MAX_GENERATED_TITLE_UNITS));
    let text = serde_json::to_string(&json!({"title":padded})).unwrap();
    assert!(matches!(
        decode(&completion(&text), &redactor),
        Err(TextError::Title(InvalidGeneratedTitle::TooLong))
    ));
}

#[test]
fn malformed_json_and_lone_surrogates_have_sealed_causes() {
    for raw in [
        r#"{"title": "private response marker""#,
        r#"{"title":"\ud800"}"#,
        r#"{"title":false}"#,
    ] {
        let error = decode(&completion(raw), &Redactor::default()).unwrap_err();
        assert!(matches!(error, TextError::Json(_)));
        let diagnostic = format!("{error} {error:?}");
        assert!(!diagnostic.contains("private response marker"));
        let source = error.source().unwrap();
        assert!(source.source().is_none());
        assert!(!source.to_string().contains("private response marker"));
    }
}

#[test]
fn final_decoded_key_is_refused_without_quoting_it() {
    let key = "synthetic-private-key-0123456789";
    let redactor = Redactor::new(Some(&ApiKey::new(key.to_owned())));
    let mut escaped = String::new();
    for character in key.chars() {
        write!(&mut escaped, "\\u{:04x}", u32::from(character)).unwrap();
    }
    let text = completion(&format!("{{\"title\":\"{escaped}\"}}"));
    let error = decode(&text, &redactor).unwrap_err();
    assert!(matches!(error, TextError::QuotedKey));
    assert!(!format!("{error} {error:?}").contains(key));
}
