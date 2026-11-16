//! Whole EN/KO arithmetic, symbols and patterns against the retained Intl run.
//! These tests are outside the active stage-1 graph and remain uncompiled.

use super::super::decimal_probe::oracle::Oracle;
use super::*;
use std::num::NonZeroUsize;

const ENGLISH: &str = include_str!("patterns/en/number-cost.ftl");
const KOREAN: &str = include_str!("patterns/ko/number-cost.ftl");
const FIRST_INEXACT_INTEGER: u64 = 9_007_199_254_740_993;

fn symbols() -> Symbols<'static> {
    Symbols::new(
        ",",
        ".",
        "0123456789",
        NonZeroUsize::new(3).unwrap(),
        NonZeroUsize::new(3).unwrap(),
    )
    .unwrap()
}

#[test]
fn all_finite_amount_rows_match_whole_locale_outputs() {
    let english = NumberKernel::new("en", ENGLISH, symbols()).unwrap();
    let korean = NumberKernel::new("ko", KOREAN, symbols()).unwrap();
    let oracle = Oracle::read().unwrap();
    for row in oracle.amounts {
        let bits = u64::from_str_radix(&row.bits, 16).unwrap();
        let value = FiniteNumber::new(f64::from_bits(bits)).unwrap();
        for (tag, kernel, expected) in [
            ("en", &english, &row.output.en),
            ("ko", &korean, &row.output.ko),
        ] {
            for (style, expected) in [
                (AmountStyle::Quantity, &expected.decimal),
                (AmountStyle::Usd, &expected.usd),
                (AmountStyle::CompactTokens, &expected.tokens),
            ] {
                assert_eq!(
                    kernel.amount(Amount::new(value, style)).unwrap(),
                    *expected,
                    "{} {tag} {style:?}",
                    row.name
                );
            }
        }
    }
}

#[test]
fn exact_count_displays_keep_all_bigint_digits() {
    let english = NumberKernel::new("en", ENGLISH, symbols()).unwrap();
    let korean = NumberKernel::new("ko", KOREAN, symbols()).unwrap();
    let oracle = Oracle::read().unwrap();
    for row in oracle.counts {
        let value = ExactCount::new(row.unsigned.parse().unwrap());
        let english = english.count(value, PluralMode::Cardinal).unwrap();
        let korean = korean.count(value, PluralMode::Cardinal).unwrap();
        assert_eq!(english.digits, row.output.en);
        assert_eq!(korean.digits, row.output.ko);
        assert_eq!(
            english.category,
            if row.unsigned == "1" {
                PluralCategory::One
            } else {
                PluralCategory::Other
            }
        );
        assert_eq!(korean.category, PluralCategory::Other);
    }
}

#[test]
fn exact_category_selection_uses_the_chosen_language_and_rule_type() {
    let english = NumberKernel::new("en", ENGLISH, symbols()).unwrap();
    let russian = NumberKernel::new("ru", ENGLISH, symbols()).unwrap();
    assert_eq!(
        english
            .count(ExactCount::new(FIRST_INEXACT_INTEGER), PluralMode::Ordinal)
            .unwrap()
            .category,
        PluralCategory::Few
    );
    assert_eq!(
        russian
            .count(ExactCount::new(9_007_199_254_741_001), PluralMode::Cardinal)
            .unwrap()
            .category,
        PluralCategory::One
    );
}

#[test]
fn presentation_keys_distinguish_signed_zero() {
    let positive = FiniteNumber::new(0.0).unwrap();
    let negative = FiniteNumber::new(-0.0).unwrap();
    assert_eq!(positive, negative);
    assert_ne!(
        Amount::new(positive, AmountStyle::Usd),
        Amount::new(negative, AmountStyle::Usd)
    );
    assert_ne!(
        Amount::new(positive, AmountStyle::Usd),
        Amount::new(positive, AmountStyle::Quantity)
    );
}
