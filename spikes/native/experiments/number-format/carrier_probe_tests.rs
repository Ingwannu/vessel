//! Unlinked integration controls for the candidate's exact Fluent boundary.
//! No test here uses formatted text as a numeric argument or plural operand.

use super::*;
use crate::placement_probe::Symbols;
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use std::num::NonZeroUsize;

const ENGLISH: &str = include_str!("patterns/en/number-cost.ftl");
const KOREAN: &str = include_str!("patterns/ko/number-cost.ftl");
const FIRST_INEXACT: u64 = 9_007_199_254_740_993;
const RUSSIAN_ONE: u64 = 9_007_199_254_741_001;

fn kernel(tag: &str, patterns: &str) -> NumberKernel<'static> {
    let symbols = Symbols::new(
        ",",
        ".",
        "0123456789",
        NonZeroUsize::new(3).unwrap(),
        NonZeroUsize::new(3).unwrap(),
    )
    .unwrap();
    NumberKernel::new(tag, patterns, symbols).unwrap()
}

fn context(tag: &str, patterns: &str) -> Arc<NumberContext> {
    NumberContext::new(
        tag.parse().unwrap(),
        kernel(tag, patterns),
        kernel("en", ENGLISH),
    )
    .unwrap()
}

fn bundle(tag: &str, context: &Arc<NumberContext>, source: &str) -> FluentBundle<FluentResource> {
    let mut bundle = FluentBundle::new_concurrent(vec![tag.parse().unwrap()]);
    bundle.set_use_isolating(false);
    let context = Arc::clone(context);
    bundle
        .add_function("NUMBER", move |positional, named| {
            context.function(positional, named)
        })
        .unwrap();
    bundle
        .add_resource(FluentResource::try_new(source.to_owned()).unwrap())
        .unwrap();
    bundle
}

fn render(bundle: &FluentBundle<FluentResource>, key: &str, value: FluentValue<'_>) -> String {
    let mut args = FluentArgs::new();
    args.set("count", value);
    let message = bundle.get_message(key).unwrap();
    let mut errors = Vec::new();
    let rendered = bundle.format_pattern(message.value().unwrap(), Some(&args), &mut errors);
    assert!(errors.is_empty(), "{key}: {errors:?}");
    rendered.into_owned()
}

#[test]
fn full_unsigned_display_and_exact_selector_survive_the_actual_number_callback() {
    let context = context("ru", ENGLISH);
    let source =
        "result = { NUMBER($count) }/{ NUMBER($count) ->\n    [one] one\n   *[other] other\n}";
    let bundle = bundle("ru", &context, source);
    let value = context
        .bind(Decimal::from(RUSSIAN_ONE), NumericOptions::default())
        .unwrap();
    assert_eq!(
        render(&bundle, "result", value),
        "9,007,199,254,741,001/one"
    );
    let maximum = context
        .bind(Decimal::from(u64::MAX), NumericOptions::default())
        .unwrap();
    assert_eq!(
        render(&bundle, "result", maximum),
        "18,446,744,073,709,551,615/other"
    );
}

#[test]
fn english_fallback_uses_english_operands_and_ordinal_options() {
    let context = context("ru", ENGLISH);
    let source =
        "result = { NUMBER($count, type: \"ordinal\") ->\n    [few] third\n   *[other] other\n}";
    let english = bundle("en", &context, source);
    let value = context
        .bind(Decimal::from(FIRST_INEXACT), NumericOptions::default())
        .unwrap();
    assert_eq!(render(&english, "result", value), "third");
}

#[test]
fn literal_order_and_nested_number_rounding_keep_numeric_operands() {
    let context = context("en", ENGLISH);
    let source = "category = { $count ->\n    [one] category\n    [1] literal\n   *[other] other\n}\nliteral = { $count ->\n    [1] literal\n    [one] category\n   *[other] other\n}\nnested = { NUMBER(NUMBER($count, maximumFractionDigits: 2), maximumFractionDigits: 1) }";
    let bundle = bundle("en", &context, source);
    for (key, expected) in [("category", "category"), ("literal", "literal")] {
        let value = context
            .bind(Decimal::from(1_u64), NumericOptions::default())
            .unwrap();
        assert_eq!(render(&bundle, key, value), expected);
    }
    let decimal = Decimal::try_from_f64(1.249, FloatPrecision::RoundTrip).unwrap();
    let value = context.bind(decimal, NumericOptions::default()).unwrap();
    assert_eq!(render(&bundle, "nested", value), "1.3");
}

#[test]
fn source_and_fallback_amount_patterns_keep_their_own_compact_scale() {
    let context = context("ko", KOREAN);
    let source = "compact = { NUMBER(NUMBER($count, notation: \"compact\", maximumFractionDigits: 1), useGrouping: \"false\") }\nusd = { NUMBER($count, style: \"currency\", currency: \"USD\") }";
    for (tag, compact) in [("en", "10K"), ("ko", "1만")] {
        let bundle = bundle(tag, &context, source);
        let value = context
            .bind(Decimal::from(10_000_u64), NumericOptions::default())
            .unwrap();
        assert_eq!(render(&bundle, "compact", value), compact);
        let negative = Decimal::try_from_f64(-0.0, FloatPrecision::RoundTrip).unwrap();
        let value = context.bind(negative, NumericOptions::default()).unwrap();
        assert_eq!(render(&bundle, "usd", value), "-$0.00");
    }
}

#[test]
fn missing_numeric_pattern_reprepares_whole_english_body_with_source_grammar() {
    let context = context("ru", "unrelated = Partial catalog");
    let source = "result = { NUMBER($count, notation: \"compact\") }/{ $count ->\n    [one] one\n   *[other] other\n}";
    let bundle = bundle("ru", &context, source);
    let value = context
        .bind(Decimal::from(21_u64), NumericOptions::default())
        .unwrap();
    assert_eq!(render(&bundle, "result", value), "21/one");
    let value = context
        .bind(Decimal::from(10_000_u64), NumericOptions::default())
        .unwrap();
    assert_eq!(render(&bundle, "result", value), "10K/other");
}
