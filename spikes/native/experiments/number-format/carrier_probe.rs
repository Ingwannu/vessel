//! Scratch exact-number carrier for the opt-in Fluent hook experiment.
//!
//! [Decision Log]
//! - Purpose: retain a numeric operand through NUMBER and ordered selectors.
//! - Constraint: FluentType's display methods cannot return a typed error.
//! - Choice: prepare source and English values before binding, and select only
//!   between those completed results using the bundle's memoized locale.
//! - Cost: a dynamic argument prepares two values, including when no fallback
//!   is used. This cost belongs to event preparation, never plain getters.

use std::{borrow::Cow, convert::Infallible, sync::Arc};

use fixed_decimal::{Decimal, FloatPrecision};
use fluent_bundle::{FluentArgs, FluentValue, types::FluentType};
use icu_plurals::PluralCategory;
use intl_memoizer::Memoizable;
use unic_langid::LanguageIdentifier;

use super::{
    kernel_probe::{KernelError, NumberKernel, PreparedNumeric, prepare_options},
    options_probe::NumericOptions,
};

#[derive(Debug)]
pub(super) struct NumberContext {
    tag: LanguageIdentifier,
    source: NumberKernel<'static>,
    english: NumberKernel<'static>,
}

impl NumberContext {
    pub(super) fn new(
        tag: LanguageIdentifier,
        source: NumberKernel<'static>,
        english: NumberKernel<'static>,
    ) -> Result<Arc<Self>, KernelError> {
        english.require_complete()?;
        Ok(Arc::new(Self {
            tag,
            source,
            english,
        }))
    }

    pub(super) fn bind(
        self: &Arc<Self>,
        value: Decimal,
        options: NumericOptions,
    ) -> Result<FluentValue<'static>, KernelError> {
        let source = prepare_options(&self.source, &value, &options)?;
        let english = prepare_options(&self.english, &value, &options)?;
        Ok(FluentValue::Custom(Box::new(NumberValue {
            context: Arc::clone(self),
            options,
            source,
            english,
        })))
    }

    pub(super) fn function<'a>(
        self: &Arc<Self>,
        positional: &[FluentValue<'a>],
        named: &FluentArgs<'_>,
    ) -> FluentValue<'a> {
        match positional.first() {
            Some(FluentValue::Custom(value)) => value
                .number_with_options(named)
                .map_or(FluentValue::Error, FluentValue::Custom),
            Some(FluentValue::Number(value)) => {
                let prepared = (|| {
                    let decimal =
                        Decimal::try_from_f64(value.value, FloatPrecision::RoundTrip).ok()?;
                    let options = NumericOptions {
                        fluent: value.options.clone(),
                        ..NumericOptions::default()
                    }
                    .merged(named)
                    .ok()?;
                    self.bind(decimal, options).ok()
                })();
                prepared.unwrap_or(FluentValue::Error)
            }
            _ => FluentValue::Error,
        }
    }
}

#[derive(Debug, Clone)]
struct NumberValue {
    context: Arc<NumberContext>,
    options: NumericOptions,
    source: PreparedNumeric,
    english: PreparedNumeric,
}

impl PartialEq for NumberValue {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.context, &other.context)
            && self.options == other.options
            && self.source == other.source
            && self.english == other.english
    }
}

impl NumberValue {
    fn prepared(&self, locale: &BundleLocale) -> &PreparedNumeric {
        match locale.0 {
            Namespace::Source => &self.source,
            Namespace::English => &self.english,
        }
    }

    fn selected(&self, selector: &FluentValue<'_>, locale: &BundleLocale) -> bool {
        let prepared = self.prepared(locale);
        match selector {
            FluentValue::String(value) => value == category(prepared.category),
            FluentValue::Number(value) => {
                Decimal::try_from_f64(value.value, FloatPrecision::RoundTrip)
                    .is_ok_and(|literal| literal == prepared.decimal)
            }
            FluentValue::Custom(_) | FluentValue::Error | FluentValue::None => false,
        }
    }
}

impl FluentType for NumberValue {
    fn duplicate(&self) -> Box<dyn FluentType + Send> {
        Box::new(self.clone())
    }

    fn number_with_options(&self, named: &FluentArgs<'_>) -> Option<Box<dyn FluentType + Send>> {
        let options = self.options.merged(named).ok()?;
        // Reprepare each language from its own previous numeric result. A
        // compact coefficient may differ between source and English bundles.
        let source = prepare_options(&self.context.source, &self.source.decimal, &options).ok()?;
        let english =
            prepare_options(&self.context.english, &self.english.decimal, &options).ok()?;
        Some(Box::new(Self {
            context: Arc::clone(&self.context),
            options,
            source,
            english,
        }))
    }

    fn matches_selector(
        &self,
        selector: &FluentValue<'_>,
        memo: &intl_memoizer::IntlLangMemoizer,
    ) -> bool {
        infallible(
            memo.with_try_get::<BundleLocale, _, _>(self.context.tag.clone(), |locale| {
                self.selected(selector, locale)
            }),
        )
    }

    fn matches_selector_threadsafe(
        &self,
        selector: &FluentValue<'_>,
        memo: &intl_memoizer::concurrent::IntlLangMemoizer,
    ) -> bool {
        infallible(
            memo.with_try_get::<BundleLocale, _, _>(self.context.tag.clone(), |locale| {
                self.selected(selector, locale)
            }),
        )
    }

    fn as_string(&self, memo: &intl_memoizer::IntlLangMemoizer) -> Cow<'static, str> {
        Cow::Owned(infallible(memo.with_try_get::<BundleLocale, _, _>(
            self.context.tag.clone(),
            |locale| self.prepared(locale).display.clone(),
        )))
    }

    fn as_string_threadsafe(
        &self,
        memo: &intl_memoizer::concurrent::IntlLangMemoizer,
    ) -> Cow<'static, str> {
        Cow::Owned(infallible(memo.with_try_get::<BundleLocale, _, _>(
            self.context.tag.clone(),
            |locale| self.prepared(locale).display.clone(),
        )))
    }
}

#[derive(Debug, Clone, Copy)]
enum Namespace {
    Source,
    English,
}

struct BundleLocale(Namespace);

impl Memoizable for BundleLocale {
    type Args = LanguageIdentifier;
    type Error = Infallible;
    fn construct(bundle: LanguageIdentifier, source: Self::Args) -> Result<Self, Self::Error> {
        Ok(Self(if bundle == source {
            Namespace::Source
        } else {
            Namespace::English
        }))
    }
}

fn infallible<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(never) => match never {},
    }
}

fn category(value: PluralCategory) -> &'static str {
    match value {
        PluralCategory::Zero => "zero",
        PluralCategory::One => "one",
        PluralCategory::Two => "two",
        PluralCategory::Few => "few",
        PluralCategory::Many => "many",
        PluralCategory::Other => "other",
    }
}

#[cfg(test)]
#[path = "carrier_probe_tests.rs"]
mod tests;
