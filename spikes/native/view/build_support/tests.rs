//! The gate distrusts translated files: fixtures exercise missing keys,
//! undeclared variables, invalid references, malformed syntax and metadata.

use std::collections::BTreeMap;
use std::path::Path;

use super::{BuildError, Language, catalog, generate, read, validate};

fn language(tag: &str, source: &str) -> Result<Language, BuildError> {
    catalog::parse(
        tag.to_owned(),
        BTreeMap::from([("meta".to_owned(), source.to_owned())]),
    )
}

fn english() -> Result<Language, BuildError> {
    language(
        "en",
        "# Native name, short.\nmeta-name = English\n# Welcome; $name is the display name.\nmeta-welcome = Welcome, { $name }.\n# Plain title, short.\nmeta-title = Home",
    )
}

fn korean() -> Result<Language, BuildError> {
    language(
        "ko",
        "meta-name = 한국어\nmeta-welcome = { $name }님, 환영합니다.\nmeta-title = 홈",
    )
}

#[test]
fn real_catalog_passes_the_gate_and_generates_an_api() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    let catalog = read(&root).unwrap();
    let coverage = validate(&catalog).unwrap();
    let generated = generate(&catalog, &coverage).unwrap();
    let _rules = super::generate_format_rules(&catalog).unwrap();
    assert!(!generated.is_empty());
    assert!(
        generated
            .lines()
            .any(|line| line == "pub fn r#welcome(self,")
    );
}

#[test]
fn korean_must_have_every_key_and_exactly_the_source_variables() {
    let en = english().unwrap();
    let missing = language("ko", "meta-name = 한국어").unwrap();
    assert_eq!(
        validate(&[en.clone(), missing])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("ko: missing messages: meta-title, meta-welcome")
    );
    let no_variable = language(
        "ko",
        "meta-name = 한국어\nmeta-welcome = 환영합니다.\nmeta-title = 홈",
    )
    .unwrap();
    assert_eq!(
        validate(&[en.clone(), no_variable])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("ko: meta-welcome must use the same variables as English")
    );
    let extra = language(
        "ko",
        "meta-name = 한국어\nmeta-welcome = { $name } { $unknown }\nmeta-title = 홈",
    )
    .unwrap();
    assert_eq!(
        validate(&[en, extra])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("ko: meta-welcome uses variables English does not pass")
    );
}

#[test]
fn a_key_in_only_one_of_english_or_korean_fails_in_both_directions() {
    for (en_only, expected) in [
        (true, "ko: missing messages: meta-only"),
        (false, "ko: unknown message meta-only"),
    ] {
        let mut en = english().unwrap();
        let mut ko = korean().unwrap();
        let extra = language("en", "# Single-language probe.\nmeta-only = Only here")
            .unwrap()
            .messages
            .remove("meta-only")
            .unwrap();
        let destination = if en_only { &mut en } else { &mut ko };
        assert!(
            destination
                .messages
                .insert("meta-only".to_owned(), extra)
                .is_none()
        );
        assert_eq!(validate(&[en, ko]).unwrap_err().to_string(), expected);
    }
}

#[test]
fn a_contributed_fixture_folder_supplies_two_messages_without_shipping() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/locales");
    let languages = read(&root).unwrap();
    let coverage = validate(&languages).unwrap();
    assert_eq!(
        coverage
            .iter()
            .map(|c| (c.tag.as_str(), c.missing.as_slice()))
            .collect::<Vec<_>>(),
        [
            ("en", &[][..]),
            (
                "ja",
                &[
                    "meta-sessions".to_owned(),
                    "meta-time".to_owned(),
                    "meta-welcome".to_owned()
                ][..]
            ),
            ("ko", &[][..])
        ]
    );
    let generated = generate(&languages, &coverage).unwrap();
    assert!(generated.lines().any(|line| line
        == "Locale { index: 1, tag: \"ja\", native_name: \"日本語\", complete: false },"));
    let extra = language("ja", "meta-name = 日本語\nmeta-welcome = { $intruder }").unwrap();
    assert_eq!(
        validate(&[english().unwrap(), korean().unwrap(), extra])
            .err()
            .map(|e| e.to_string())
            .as_deref(),
        Some("ja: meta-welcome uses variables English does not pass")
    );
}

#[test]
fn references_cannot_hide_an_extra_variable_or_a_cycle() {
    let extra = language(
        "ja",
        "-hidden = { $intruder }\nmeta-name = 日本語\nmeta-welcome = { -hidden }",
    )
    .unwrap();
    assert_eq!(
        validate(&[english().unwrap(), korean().unwrap(), extra])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("ja: -hidden needs a locally bound $intruder parameter")
    );
    let cycle = language(
        "ja",
        "meta-name = 日本語\nmeta-welcome = { meta-title }\nmeta-title = { meta-welcome }",
    )
    .unwrap();
    assert_eq!(
        validate(&[english().unwrap(), korean().unwrap(), cycle])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("ja: cyclic or excessively deep reference at meta-title")
    );
    let missing = language("ja", "meta-name = 日本語\nmeta-title = { meta-welcome }").unwrap();
    assert_eq!(
        validate(&[english().unwrap(), korean().unwrap(), missing])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("ja: missing reference meta-welcome; translate it too or omit the referring message")
    );
}

#[test]
fn malformed_duplicates_unknown_keys_and_missing_context_fail() {
    assert!(language("en", "meta-name = { broken").is_err());
    assert_eq!(
        language("en", "meta-name = English\nmeta-name = Again")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("en/meta.ftl: duplicate message meta-name")
    );
    let unknown = language("ja", "meta-name = 日本語\nmeta-extra = 余計").unwrap();
    assert_eq!(
        validate(&[english().unwrap(), korean().unwrap(), unknown])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("ja: unknown message meta-extra")
    );
    let no_context = language("en", "meta-name = English").unwrap();
    assert_eq!(
        validate(&[no_context, korean().unwrap()])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("en: meta-name needs a translator context comment")
    );
    let no_name = language("ja", "meta-title = ホーム").unwrap();
    assert_eq!(
        validate(&[english().unwrap(), korean().unwrap(), no_name])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("ja: meta-name must be a nonempty plain native language name")
    );
}

#[test]
fn one_variable_cannot_require_two_incompatible_caller_types() {
    let bad = language(
        "en",
        "# Own name.\nmeta-name = English\n# Invalid mixed value.\nmeta-welcome = { NUMBER($value) } { DATETIME($value) }",
    ).unwrap();
    assert_eq!(
        validate(&[bad, korean().unwrap()])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("en: meta-welcome uses incompatible types for $value")
    );
}

#[test]
fn grammar_calls_have_checked_names_and_arguments() {
    for (source, expected) in [
        (
            "meta-name = English\nmeta-welcome = { MAGIC($name) }",
            "en/meta.ftl: unknown function MAGIC",
        ),
        (
            "meta-name = English\nmeta-welcome = { NUMBER() }",
            "en/meta.ftl: NUMBER needs 1 positional arguments and no named options",
        ),
        (
            "meta-name = English\nmeta-welcome = { NUMBER($count, minimumFractionDigits: 5, maximumFractionDigits: 2) }",
            "en/meta.ftl: minimumFractionDigits exceeds maximumFractionDigits",
        ),
        (
            "meta-name = English\nmeta-welcome = { NUMBER(4294967296) }",
            "en/meta.ftl: numeric literals require u32 magnitude and at most nine decimal places",
        ),
        (
            "meta-name = English\nmeta-welcome = { DATETIME(\"bad\") }",
            "en/meta.ftl: DATETIME requires a typed date variable",
        ),
        (
            "meta-name = English\nmeta-welcome = { PARTICLE($name, \"bad\") }",
            "en/meta.ftl: PARTICLE's second argument must name object, subject, topic, with, or direction",
        ),
    ] {
        assert_eq!(
            language("en", source)
                .err()
                .map(|error| error.to_string())
                .as_deref(),
            Some(expected)
        );
    }
}

#[test]
fn resources_must_be_regular_files() {
    let root = Path::new(env!("OUT_DIR")).join(format!("locale-file-kind-{}", std::process::id()));
    std::fs::create_dir_all(root.join("en")).unwrap();
    std::fs::create_dir_all(root.join("ko")).unwrap();
    std::fs::write(root.join("en/meta.ftl"), "# Own name.\nmeta-name = English").unwrap();
    std::fs::write(root.join("ko/meta.ftl"), "meta-name = 한국어").unwrap();
    std::fs::create_dir(root.join("en/invalid.ftl")).unwrap();
    let result = read(&root);
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(
        result.err().map(|error| error.to_string()).as_deref(),
        Some("en: translation resources must be regular files, not links or directories")
    );
}
