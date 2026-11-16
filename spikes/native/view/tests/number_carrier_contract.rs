//! Negative controls for ADR 0101's rejected numeric carrier. These deliberately
//! exercise the unextended upstream bundle, not Vessel's future exact adapter.
//! A lossless display hook alone cannot repair a rounded selector operand.

use fluent_bundle::{
    FluentArgs, FluentResource, FluentValue,
    concurrent::FluentBundle,
    types::{FluentNumber, FluentNumberType},
};

#[test]
fn stock_fluent_collapses_distinct_unsigned_integers() {
    const FIRST: u64 = 9_007_199_254_740_992;
    const NEXT: u64 = 9_007_199_254_740_993;

    assert_ne!(FIRST, NEXT);
    let FluentValue::Number(first) = FluentValue::from(FIRST) else {
        panic!("upstream changed its unsigned carrier; reassess ADR 0101");
    };
    let FluentValue::Number(next) = FluentValue::from(NEXT) else {
        panic!("upstream changed its unsigned carrier; reassess ADR 0101");
    };
    assert_eq!(first.value.to_bits(), next.value.to_bits());
    assert_eq!(first.as_string(), "9007199254740992");
    assert_eq!(next.as_string(), "9007199254740992");
}

#[test]
fn stock_fluent_selects_plurals_from_the_rounded_unsigned_operand() {
    // Russian's integer ending in 1, except 11, selects "one". The last
    // decimal digit here disappears at the upstream u64 -> f64 boundary.
    const COUNT: u64 = 9_007_199_254_741_001;
    const SOURCE: &str = "observed = { $count } / { $count ->\n    [one] one\n    [few] few\n    [many] many\n   *[other] other\n}";
    let language = "ru".parse().unwrap();
    let mut bundle = FluentBundle::new_concurrent(vec![language]);
    bundle.set_use_isolating(false);
    bundle
        .add_resource(FluentResource::try_new(SOURCE.to_owned()).unwrap())
        .unwrap();
    let message = bundle.get_message("observed").unwrap();
    let pattern = message.value().unwrap();

    // Positive controls distinguish a working plural engine from a broken
    // fixture. The large case characterizes why this carrier is rejected.
    for (count, expected) in [
        (1, "1 / one"),
        (11, "11 / many"),
        (21, "21 / one"),
        (COUNT, "9007199254741000 / many"),
    ] {
        let mut args = FluentArgs::new();
        args.set("count", count);
        let mut errors = Vec::new();
        let observed = bundle.format_pattern(pattern, Some(&args), &mut errors);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(observed, expected);
    }
}

#[test]
fn english_ordinals_also_expose_the_unsigned_rounding_boundary() {
    // Shipped EN/KO messages currently use cardinals, which do not change
    // category above this boundary. The supported ordinal option is different:
    // English 93 is "few", whereas the rounded integer ends in 92 ("two").
    const COUNT: u64 = 9_007_199_254_740_993;
    const SOURCE: &str =
        "observed = { $count ->\n    [one] one\n    [two] two\n    [few] few\n   *[other] other\n}";
    let language = "en".parse().unwrap();
    let mut bundle = FluentBundle::new_concurrent(vec![language]);
    bundle
        .add_resource(FluentResource::try_new(SOURCE.to_owned()).unwrap())
        .unwrap();
    let message = bundle.get_message("observed").unwrap();
    let pattern = message.value().unwrap();
    for (count, expected) in [(3, "few"), (13, "other"), (COUNT, "two")] {
        let mut value = FluentNumber::from(count);
        value.options.r#type = FluentNumberType::Ordinal;
        let mut args = FluentArgs::new();
        args.set("count", value);
        let mut errors = Vec::new();
        let observed = bundle.format_pattern(pattern, Some(&args), &mut errors);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(observed, expected);
    }
}
