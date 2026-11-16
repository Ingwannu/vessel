//! Candidate-only integration checks, registered by the measurement overlay.
//! These exercise generated getters and the actual core resolver/callback.

use super::*;
use crate::decimal_probe::oracle::Oracle;
use vessel_proto::conversation::FiniteNumber;

#[test]
fn generated_amount_arguments_match_every_whole_intl_sentence() {
    let oracle = Oracle::read_extended().unwrap();
    for row in oracle.amounts {
        let bits = u64::from_str_radix(&row.bits, 16).unwrap();
        let value = FiniteNumber::new(f64::from_bits(bits)).unwrap();
        for (tag, output) in [("en", &row.output.en), ("ko", &row.output.ko)] {
            let words = Translator::get(Locale::from_tag(tag).unwrap()).unwrap();
            for (unit, expected) in [
                (AmountUnit::Usd, &output.usd),
                (AmountUnit::CompactTokens, &output.tokens),
                (AmountUnit::Quantity, &output.decimal),
            ] {
                let expected = match tag {
                    "en" => format!("\u{2068}{expected}\u{2069} sessions in total."),
                    _ => format!("세션 총 \u{2068}{expected}\u{2069}개입니다."),
                };
                assert_eq!(
                    words
                        .examples()
                        .grouped(Number::amount(value, unit))
                        .unwrap(),
                    expected,
                    "{} {tag} {unit:?}",
                    row.name,
                );
            }
        }
    }
}

#[test]
fn generated_count_arguments_preserve_every_unsigned_oracle_row() {
    let oracle = Oracle::read_extended().unwrap();
    for row in oracle.counts {
        let count = row.unsigned.parse::<u64>().unwrap();
        for (tag, digits) in [("en", &row.output.en), ("ko", &row.output.ko)] {
            let words = Translator::get(Locale::from_tag(tag).unwrap()).unwrap();
            let expected = match tag {
                "en" => format!("\u{2068}{digits}\u{2069} sessions in total."),
                _ => format!("세션 총 \u{2068}{digits}\u{2069}개입니다."),
            };
            assert_eq!(
                words.examples().grouped(Number::unsigned(count)).unwrap(),
                expected
            );
        }
    }
}

#[test]
fn whole_app_message_fallback_reselects_english_numeric_grammar() {
    const COUNT: u64 = 9_007_199_254_741_001;
    let source = "other = Partial source";
    let partial = runtime::fixture("ru", source).unwrap();
    let english = runtime::fixture(
        "en",
        "result = { NUMBER($count) ->\n [one] English one\n *[other] English other\n}",
    )
    .unwrap();
    let context = numeric_runtime::context("ru", &[source]).unwrap();
    let mut arguments = FluentArgs::new();
    arguments.set("count", Number::unsigned(COUNT).fluent(&context).unwrap());
    assert_eq!(
        resolve(&partial, &english, "result", Some(&arguments)).unwrap(),
        "English other"
    );
}

#[test]
fn public_numeric_keys_retain_unit_and_signed_zero() {
    let positive = FiniteNumber::new(0.0).unwrap();
    let negative = FiniteNumber::new(-0.0).unwrap();
    assert_ne!(
        Number::amount(positive, AmountUnit::Usd),
        Number::amount(negative, AmountUnit::Usd)
    );
    assert_ne!(
        Number::amount(positive, AmountUnit::Usd),
        Number::amount(positive, AmountUnit::Quantity)
    );
}
