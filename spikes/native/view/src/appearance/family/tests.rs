//! Upstream sanitizer edge cases, including JavaScript UTF-16 length.
use super::*;

#[test]
fn retains_valid_spelling_and_accepts_the_explicit_default() {
    for (raw, expected) in [
        (" \t ", ""),
        ("  Geist, Pretendard  ", "Geist, Pretendard"),
        ("'A / B (Book)', sans-serif", "'A / B (Book)', sans-serif"),
        (r#""A\"B", serif"#, r#""A\"B", serif"#),
        ("한글 글꼴", "한글 글꼴"),
        ("\u{feff}Geist\u{feff}", "Geist"),
        ("\u{85}Geist\u{85}", "\u{85}Geist\u{85}"),
        ("A\u{7f}B", "A\u{7f}B"),
    ] {
        assert_eq!(FontFamily::parse(raw).unwrap().as_str(), expected);
    }
}

#[test]
fn rejects_declaration_controls_and_unfinished_constructs() {
    for raw in [
        "A;B", "A{B}", "<font>", "A\nB", "A\0B", "'A", "A\\", "A/*B", "A(B)", "A[B]",
    ] {
        assert!(FontFamily::parse(raw).is_err(), "{raw:?}");
    }
    assert!(matches!(
        FontFamily::parse("'A;B'"),
        Err(FamilyError::Forbidden)
    ));
}

#[test]
fn counts_utf16_units_and_serde_cannot_bypass_the_invariant() {
    let limit = "x".repeat(MAX_FAMILY_UNITS);
    assert!(FontFamily::parse(&limit).is_ok());
    assert!(FontFamily::parse(&(limit + "x")).is_err());
    let supplementary = "😀".repeat(MAX_FAMILY_UNITS / 2);
    assert!(FontFamily::parse(&supplementary).is_ok());
    assert!(FontFamily::parse(&(supplementary + "x")).is_err());
    assert!(serde_json::from_str::<FontFamily>(r#""A;B""#).is_err());
    let family = FontFamily::parse("Geist").unwrap();
    assert_eq!(
        serde_json::from_str::<FontFamily>(&serde_json::to_string(&family).unwrap()).unwrap(),
        family
    );
}
