use super::*;

#[test]
fn original_native_forms_keep_their_grammar_and_canonical_spelling() {
    for (raw, expected) in [
        ("en", "en"),
        ("EN", "en"),
        ("ko-KR", "ko-KR"),
        ("EN_us", "en-US"),
        ("sr_Cyrl_RS", "sr-Cyrl-RS"),
        ("de-Latn-DE-1996", "de-Latn-DE-1996"),
        ("sl-rozaj-biske", "sl-biske-rozaj"),
        ("en-XA", "en-XA"),
    ] {
        let tag: LocaleTag = raw.parse().expect("native accepted form");
        assert_eq!(tag.as_str(), expected);
        let encoded = serde_json::to_string(&tag).expect("serialize tag");
        assert_eq!(encoded, format!("\"{expected}\""));
        assert_eq!(
            serde_json::from_str::<LocaleTag>(&encoded).expect("tag roundtrip"),
            tag
        );
    }
    for raw in [
        "",
        " ",
        " en",
        "en ",
        "en--US",
        "en-u-ca-gregory",
        "bad!",
        "ko/PRIVATE-LOCALE-CANARY",
    ] {
        assert!(matches!(
            raw.parse::<LocaleTag>(),
            Err(InvalidLocaleTag::Syntax(_))
        ));
    }
}
#[test]
fn exact_utf8_byte_boundary_precedes_the_same_native_parser() {
    let exact = format!(
        "en-{}",
        (0..21)
            .map(|index| format!("v{index:04}"))
            .collect::<Vec<_>>()
            .join("-")
    );
    assert_eq!(exact.len(), MAX_LOCALE_TAG_BYTES);
    assert!(exact.parse::<LocaleTag>().is_ok());
    assert!(matches!(
        format!("{exact}-v0021").parse::<LocaleTag>(),
        Err(InvalidLocaleTag::Capacity)
    ));
    assert!(matches!(
        "é".repeat(65).parse::<LocaleTag>(),
        Err(InvalidLocaleTag::Capacity)
    ));
    assert!(matches!(
        "é".parse::<LocaleTag>(),
        Err(InvalidLocaleTag::Syntax(_))
    ));
    assert_eq!(
        "EN".parse::<LocaleTag>(),
        "en".parse::<LocaleTag>(),
        "resource owner sees one canonical duplicate key"
    );
}
#[test]
fn invalid_tag_diagnostics_and_serde_never_reopen_input() {
    let input = "ko/PRIVATE-LOCALE-CANARY";
    let error = input.parse::<LocaleTag>().expect_err("invalid native form");
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    while let Some(error) = source {
        assert!(!format!("{error:?} {error}").contains("PRIVATE-LOCALE-CANARY"));
        source = error.source();
    }
    let encoded = serde_json::to_string(input).expect("fixture string");
    let error = serde_json::from_str::<LocaleTag>(&encoded).expect_err("serde refuses");
    assert!(!format!("{error:?} {error}").contains("PRIVATE-LOCALE-CANARY"));
}
#[cfg(feature = "ts")]
#[test]
fn typescript_is_the_same_canonical_string_edge() {
    assert_eq!(
        <LocaleTag as ts_rs::TS>::inline(&ts_rs::Config::new()),
        "string"
    );
}
