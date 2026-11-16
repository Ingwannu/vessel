//! These tests are NOT part of the product target until the controlled
//! experiment imports the candidate; an ordinary View green cannot cover them.

use super::*;
use std::collections::BTreeMap;

const ORACLE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/number-format/intl-en-ko.json"
));
const EN_SCALES: &[i16] = &[3, 6, 9, 12];
const KO_SCALES: &[i16] = &[3, 4, 8, 12];

#[derive(serde::Deserialize)]
struct Oracle {
    amounts: Vec<Amount>,
    counts: Vec<Count>,
}

#[derive(serde::Deserialize)]
struct Amount {
    name: String,
    bits: String,
    output: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(serde::Deserialize)]
struct Count {
    unsigned: String,
}

// The oracle fixture uses Latin digits in EN/KO. Removing its locale
// decoration compares only arithmetic; it does not test our grouping/patterns.
fn digits(value: &str) -> String {
    value
        .chars()
        .filter(|value| value.is_ascii_digit() || matches!(value, '-' | '.'))
        .collect()
}

#[test]
fn finite_rounding_matches_recorded_intl_amounts() {
    let oracle: Oracle = serde_json::from_str(ORACLE).unwrap();
    for row in oracle.amounts {
        let bits = u64::from_str_radix(&row.bits, 16).unwrap();
        let input = FiniteNumber::new(f64::from_bits(bits)).unwrap();
        for (style, presentation) in [
            ("usd", DecimalPresentation::Usd),
            ("decimal", DecimalPresentation::Quantity),
        ] {
            let expected = digits(&row.output["en"][style]);
            assert_eq!(
                rounded(input, presentation).unwrap().to_string(),
                expected,
                "{} {style}",
                row.name
            );
        }
    }
}

#[test]
fn compact_coefficients_follow_each_scale_and_carry() {
    let oracle: Oracle = serde_json::from_str(ORACLE).unwrap();
    for row in oracle.amounts {
        let bits = u64::from_str_radix(&row.bits, 16).unwrap();
        let input = FiniteNumber::new(f64::from_bits(bits)).unwrap();
        for (language, scales) in [("en", EN_SCALES), ("ko", KO_SCALES)] {
            let expected = digits(&row.output[language]["tokens"]);
            let scales = CompactScales::new(scales).unwrap();
            let prepared = compact(input, &scales).unwrap();
            assert_eq!(
                prepared.value().to_string(),
                expected,
                "{} {language}",
                row.name
            );
            assert!(
                scales.0.contains(&prepared.exponent().get()) || prepared.exponent().get() == 0
            );
        }
    }
}

#[test]
fn exact_unsigned_conversion_matches_bigint_inputs() {
    let oracle: Oracle = serde_json::from_str(ORACLE).unwrap();
    for row in oracle.counts {
        let input = ExactCount::new(row.unsigned.parse().unwrap());
        assert_eq!(input.decimal().to_string(), row.unsigned);
    }
}

#[test]
fn invalid_scale_data_is_rejected_before_formatting() {
    assert_eq!(
        CompactScales::new(&[-1]).unwrap_err(),
        ScaleError::OutOfRange
    );
    assert_eq!(
        CompactScales::new(&[309]).unwrap_err(),
        ScaleError::OutOfRange
    );
    assert_eq!(
        CompactScales::new(&[3, 3]).unwrap_err(),
        ScaleError::NotIncreasing
    );
    assert_eq!(
        CompactScales::new(&[4, 3]).unwrap_err(),
        ScaleError::NotIncreasing
    );
}
