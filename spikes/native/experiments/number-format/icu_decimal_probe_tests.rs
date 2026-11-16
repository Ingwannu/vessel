//! Exact Intl corpus comparison for the unlinked ICU experiment. USD tests
//! cover the formatted numeric body only; common currency patterns are owed.

use super::super::decimal_probe::oracle::Oracle;
use super::*;

#[test]
fn locale_decimal_bodies_and_compact_strings_match_intl() {
    let english = IcuNumbers::new("en".parse().unwrap()).unwrap();
    let korean = IcuNumbers::new("ko".parse().unwrap()).unwrap();
    let oracle = Oracle::read().unwrap();
    for row in oracle.amounts {
        let bits = u64::from_str_radix(&row.bits, 16).unwrap();
        let input = FiniteNumber::new(f64::from_bits(bits)).unwrap();
        for (language, engine, expected) in [
            ("en", &english, &row.output.en),
            ("ko", &korean, &row.output.ko),
        ] {
            assert_eq!(
                engine.amount(input, DecimalPresentation::Quantity).unwrap(),
                expected.decimal,
                "{} {language} decimal",
                row.name
            );
            let currency_body = expected.usd.replace("US$", "").replace('$', "");
            assert_eq!(
                engine.amount(input, DecimalPresentation::Usd).unwrap(),
                currency_body,
                "{} {language} USD numeric body",
                row.name
            );
            assert_eq!(
                engine.compact(input).unwrap(),
                expected.tokens,
                "{} {language} compact",
                row.name
            );
        }
    }
}

#[test]
fn grouped_unsigned_counts_match_bigint_exactly() {
    let english = IcuNumbers::new("en".parse().unwrap()).unwrap();
    let korean = IcuNumbers::new("ko".parse().unwrap()).unwrap();
    let oracle = Oracle::read().unwrap();
    for row in oracle.counts {
        let input = ExactCount::new(row.unsigned.parse().unwrap());
        assert_eq!(english.count(input), row.output.en);
        assert_eq!(korean.count(input), row.output.ko);
    }
}
