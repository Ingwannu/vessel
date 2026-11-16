//! Actual whole outputs distinguish fallback from mixed source/English parts.

use super::super::{kernel_probe::AmountStyle, placement_probe::Symbols};
use super::*;
use std::num::NonZeroUsize;
use vessel_proto::conversation::FiniteNumber;

const ENGLISH: &str = include_str!("patterns/en/number-cost.ftl");
const PARTIAL: &str =
    "number-cost-quantity = { $sign ->\n [negative] −{ $absolute }\n *[positive] { $absolute }\n}";

fn symbols<'a>(group: &'a str, decimal: &'a str) -> Symbols<'a> {
    Symbols::new(
        group,
        decimal,
        "0123456789",
        NonZeroUsize::new(3).unwrap(),
        NonZeroUsize::new(3).unwrap(),
    )
    .unwrap()
}

#[test]
fn each_missing_pattern_uses_the_whole_english_number() {
    let selected = NumberKernel::new("ko", PARTIAL, symbols(".", ",")).unwrap();
    let english = NumberKernel::new("en", ENGLISH, symbols(",", ".")).unwrap();
    let numbers = FallbackNumbers::new(selected, english).unwrap();
    let value = FiniteNumber::new(1234.5).unwrap();
    assert_eq!(
        numbers
            .amount(Amount::new(value, AmountStyle::Quantity))
            .unwrap(),
        "1.234,5"
    );
    assert_eq!(
        numbers
            .amount(Amount::new(value, AmountStyle::Usd))
            .unwrap(),
        "$1,234.50"
    );
    let value = FiniteNumber::new(10_000.0).unwrap();
    assert_eq!(
        numbers
            .amount(Amount::new(value, AmountStyle::CompactTokens))
            .unwrap(),
        "10K"
    );
}

#[test]
fn an_incomplete_english_baseline_is_rejected() {
    let selected = NumberKernel::new("ko", PARTIAL, symbols(".", ",")).unwrap();
    let english = NumberKernel::new("en", PARTIAL, symbols(",", ".")).unwrap();
    assert!(FallbackNumbers::new(selected, english).is_err());
}
