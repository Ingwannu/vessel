//! These tests are NOT part of the product target until the controlled
//! experiment imports the candidate; an ordinary View green cannot cover them.

use super::oracle::Oracle;
use super::*;

const EN_SCALES: &[i16] = &[3, 6, 9, 12];
const KO_SCALES: &[i16] = &[3, 4, 8, 12];

// The oracle fixture uses Latin digits in EN/KO. Removing its locale
// decoration compares only arithmetic; it does not test our grouping/patterns.
fn digits(value: &str) -> String {
    value
        .chars()
        .filter(|value| value.is_ascii_digit() || matches!(*value, '-' | '.'))
        .collect()
}

#[test]
fn finite_rounding_matches_recorded_intl_amounts() {
    let oracle = Oracle::read().unwrap();
    for row in oracle.amounts {
        let bits = u64::from_str_radix(&row.bits, 16).unwrap();
        let input = FiniteNumber::new(f64::from_bits(bits)).unwrap();
        for (expected, presentation) in [
            (&row.output.en.usd, DecimalPresentation::Usd),
            (&row.output.en.decimal, DecimalPresentation::Quantity),
        ] {
            let expected = digits(expected);
            assert_eq!(
                rounded(input, presentation).unwrap().to_string(),
                expected,
                "{} {presentation:?}",
                row.name
            );
        }
    }
}

#[test]
fn compact_coefficients_follow_each_scale_and_carry() {
    let oracle = Oracle::read().unwrap();
    for row in oracle.amounts {
        let bits = u64::from_str_radix(&row.bits, 16).unwrap();
        let input = FiniteNumber::new(f64::from_bits(bits)).unwrap();
        for (language, output, scales) in [
            ("en", &row.output.en, EN_SCALES),
            ("ko", &row.output.ko, KO_SCALES),
        ] {
            let expected = digits(&output.tokens);
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
    let oracle = Oracle::read().unwrap();
    for row in oracle.counts {
        let input = ExactCount::new(row.unsigned.parse().unwrap());
        assert_eq!(input.decimal().to_string(), row.unsigned);
        assert_eq!(digits(&row.output.en), row.unsigned);
        assert_eq!(digits(&row.output.ko), row.unsigned);
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
