//! Companion integration-test input for the patched upstream crate, not view.
//! The mock projects known categories to test dispatch/options/order only.
//! It is not a plural-rule implementation or a full-range numeric proof.

use std::borrow::Cow;

use fluent_bundle::{
    FluentArgs, FluentResource, FluentValue,
    bundle::FluentBundle,
    memoizer::MemoizerKind,
    types::{FluentNumberOptions, FluentNumberType, FluentType},
};

#[derive(Debug, Clone, PartialEq)]
struct ProbeNumber(FluentNumberOptions);

impl ProbeNumber {
    fn selected(&self, selector: &FluentValue<'_>) -> bool {
        match selector {
            FluentValue::String(value) => {
                let expected = match self.0.r#type {
                    FluentNumberType::Cardinal => "one",
                    FluentNumberType::Ordinal => "few",
                };
                value == expected
            }
            FluentValue::Number(value) => value.value.to_bits() == f64::from(1).to_bits(),
            FluentValue::Custom(_) | FluentValue::Error | FluentValue::None => false,
        }
    }
}

impl FluentType for ProbeNumber {
    fn duplicate(&self) -> Box<dyn FluentType + Send> {
        Box::new(self.clone())
    }

    fn number_with_options(&self, options: &FluentArgs<'_>) -> Option<Box<dyn FluentType + Send>> {
        let mut value = self.clone();
        value.0.merge(options);
        Some(Box::new(value))
    }

    fn matches_selector(
        &self,
        selector: &FluentValue<'_>,
        _intls: &intl_memoizer::IntlLangMemoizer,
    ) -> bool {
        self.selected(selector)
    }

    fn matches_selector_threadsafe(
        &self,
        selector: &FluentValue<'_>,
        _intls: &intl_memoizer::concurrent::IntlLangMemoizer,
    ) -> bool {
        self.selected(selector)
    }

    fn as_string(&self, _intls: &intl_memoizer::IntlLangMemoizer) -> Cow<'static, str> {
        Cow::Borrowed("exact")
    }

    fn as_string_threadsafe(
        &self,
        _intls: &intl_memoizer::concurrent::IntlLangMemoizer,
    ) -> Cow<'static, str> {
        Cow::Borrowed("exact")
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ProbeText;

impl FluentType for ProbeText {
    fn duplicate(&self) -> Box<dyn FluentType + Send> {
        Box::new(self.clone())
    }

    fn as_string(&self, _intls: &intl_memoizer::IntlLangMemoizer) -> Cow<'static, str> {
        Cow::Borrowed("one")
    }

    fn as_string_threadsafe(
        &self,
        _intls: &intl_memoizer::concurrent::IntlLangMemoizer,
    ) -> Cow<'static, str> {
        Cow::Borrowed("one")
    }
}

const SOURCE: &str = "display = { $count }
direct = { $count ->
    [one] accepted
   *[other] rejected
}
options = { NUMBER($count, type: \"ordinal\") ->
    [few] accepted
   *[other] rejected
}
nested = { NUMBER(NUMBER($count, type: \"ordinal\"), useGrouping: \"false\") ->
    [few] accepted
   *[other] rejected
}
category-first = { $count ->
    [one] category
    [1] literal
   *[other] rejected
}
literal-first = { $count ->
    [1] literal
    [one] category
   *[other] rejected
}
opaque = { $text ->
    [one] rejected
   *[other] accepted
}
stock = { $stock ->
    [1] accepted
   *[other] rejected
}";

fn verify<M: MemoizerKind>(mut bundle: FluentBundle<FluentResource, M>) {
    bundle.set_use_isolating(false);
    bundle
        .add_resource(FluentResource::try_new(SOURCE.to_owned()).unwrap())
        .unwrap();
    bundle
        .add_function("NUMBER", |positional, named| {
            let Some(FluentValue::Custom(value)) = positional.first() else {
                return FluentValue::Error;
            };
            value
                .number_with_options(named)
                .map_or(FluentValue::Error, FluentValue::Custom)
        })
        .unwrap();
    let mut args = FluentArgs::new();
    args.set(
        "count",
        FluentValue::Custom(Box::new(ProbeNumber(FluentNumberOptions::default()))),
    );
    args.set("text", FluentValue::Custom(Box::new(ProbeText)));
    args.set("stock", 1);
    for (key, expected) in [
        ("display", "exact"),
        ("direct", "accepted"),
        ("options", "accepted"),
        ("nested", "accepted"),
        ("category-first", "category"),
        ("literal-first", "literal"),
        ("opaque", "accepted"),
        ("stock", "accepted"),
    ] {
        let message = bundle.get_message(key).unwrap();
        let pattern = message.value().unwrap();
        let mut errors = Vec::new();
        let value = bundle.format_pattern(pattern, Some(&args), &mut errors);
        assert!(errors.is_empty(), "{key}: {errors:?}");
        assert_eq!(value, expected, "{key}");
    }
}

#[test]
fn ordinary_memoizer_dispatches_custom_numeric_hooks() {
    let language = "en".parse().unwrap();
    verify(FluentBundle::new(vec![language]));
}

#[test]
fn concurrent_memoizer_dispatches_custom_numeric_hooks() {
    let language = "en".parse().unwrap();
    verify(fluent_bundle::concurrent::FluentBundle::new_concurrent(
        vec![language],
    ));
}
