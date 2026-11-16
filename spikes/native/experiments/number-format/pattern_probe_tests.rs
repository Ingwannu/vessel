//! Whole rendered amounts; no substring assertions or locale table in code.

use super::super::decimal_probe::{ExactCount, compact};
use super::*;
use vessel_proto::conversation::FiniteNumber;

const ENGLISH: &str = include_str!("patterns/en/number-cost.ftl");
const KOREAN: &str = include_str!("patterns/ko/number-cost.ftl");

#[test]
fn signs_and_currency_patterns_are_whole_file_messages() {
    for (source, positive, negative) in [
        (ENGLISH, "$1,234.50", "-$1,234.50"),
        (KOREAN, "US$1,234.50", "-US$1,234.50"),
    ] {
        let patterns = Patterns::new("en", source).unwrap();
        assert_eq!(
            patterns
                .render(NumberParts::Usd {
                    absolute: "1,234.50",
                    sign: NumberSign::Positive
                })
                .unwrap(),
            positive
        );
        assert_eq!(
            patterns
                .render(NumberParts::Usd {
                    absolute: "1,234.50",
                    sign: NumberSign::Negative
                })
                .unwrap(),
            negative
        );
        assert_eq!(
            patterns
                .render(NumberParts::Quantity {
                    absolute: "0",
                    sign: NumberSign::Negative
                })
                .unwrap(),
            "-0"
        );
    }
}

#[test]
fn compact_scale_and_words_are_read_from_each_file() {
    let value = FiniteNumber::new(10_000.0).unwrap();
    for (source, expected) in [(ENGLISH, "10K"), (KOREAN, "1만")] {
        let patterns = Patterns::new("en", source).unwrap();
        let scales = patterns.scales().unwrap();
        let prepared = compact(value, &scales).unwrap();
        assert_eq!(
            patterns
                .render(NumberParts::Compact {
                    absolute: &prepared.value().to_string(),
                    sign: NumberSign::Positive,
                    exponent: prepared.exponent(),
                    category: PluralCategory::Other,
                })
                .unwrap(),
            expected
        );
    }
}

#[test]
fn all_categories_have_an_explicit_whole_amount_branch() {
    let patterns = Patterns::new("en", ENGLISH).unwrap();
    let value = ExactCount::new(1).decimal();
    let absolute = value.to_string();
    for category in PluralCategory::all() {
        assert_eq!(
            patterns
                .render(NumberParts::Compact {
                    absolute: &absolute,
                    sign: NumberSign::Positive,
                    exponent: CompactExponent::from(3),
                    category,
                })
                .unwrap(),
            "1K"
        );
    }
}

#[test]
fn scale_data_errors_are_rejected_during_construction() {
    let invalid = ENGLISH.replace("[12]", "[309]");
    assert!(matches!(
        Patterns::new("en", &invalid),
        Err(PatternError::Scales(_))
    ));
    let duplicate = ENGLISH.replace("[12]", "[3]");
    assert!(matches!(
        Patterns::new("en", &duplicate),
        Err(PatternError::DuplicateScale(3))
    ));
    let wrong_default = ENGLISH.replace("*[0]", "*[2]");
    assert!(matches!(
        Patterns::new("en", &wrong_default),
        Err(PatternError::DefaultScale)
    ));
}
