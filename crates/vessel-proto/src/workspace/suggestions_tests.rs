//! Raw admission and JS-compatible edge trim share direct/serde fixtures.
use super::{MAX_SUGGESTION_QUERY_CHARS, SuggestionQuery};

fn decoded(text: &str) -> Result<SuggestionQuery, serde_json::Error> {
    serde_json::from_str(&serde_json::to_string(text)?)
}

#[test]
fn bom_and_unicode_spaces_trim_but_interior_text_is_unchanged() {
    for (raw, expected) in [
        ("", ""),
        (" \u{feff} ", ""),
        ("\u{feff}한글🙂\u{feff}", "한글🙂"),
        ("\u{00a0}a b\u{2003}", "a b"),
        ("a\u{feff}b", "a\u{feff}b"),
    ] {
        assert_eq!(
            SuggestionQuery::try_from(raw.to_owned()).unwrap().as_str(),
            expected
        );
        assert_eq!(decoded(raw).unwrap().as_str(), expected);
    }
}

#[test]
fn resource_floor_counts_raw_scalars_and_never_trims_prohibited_controls() {
    let boundary = "🙂".repeat(MAX_SUGGESTION_QUERY_CHARS);
    assert!(SuggestionQuery::try_from(boundary.clone()).is_ok());
    assert!(decoded(&boundary).is_ok());
    for raw in [format!("{boundary}a"), format!("\u{feff}{boundary}")] {
        assert!(SuggestionQuery::try_from(raw.clone()).is_err());
        assert!(decoded(&raw).is_err());
    }
    for control in ['\0', '\t', '\r', '\n', '\u{0085}', '\u{2028}', '\u{2029}'] {
        for raw in [format!("{control}a{control}"), format!("a{control}b")] {
            assert!(SuggestionQuery::try_from(raw.clone()).is_err());
            assert!(decoded(&raw).is_err());
        }
    }
}
