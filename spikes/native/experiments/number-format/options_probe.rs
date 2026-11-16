//! Checked NUMBER options for the scratch carrier, not a product API.
//! Keep the existing nine-place message contract; full-finite amount policies
//! use the same bounded options without bounding their numeric input.

use fluent_bundle::{FluentArgs, FluentValue, types::FluentNumberOptions};

const MAX_MESSAGE_DIGITS: u8 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Notation {
    Ordinary,
    Compact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NumericOptions {
    pub(super) fluent: FluentNumberOptions,
    pub(super) notation: Notation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("unsupported experimental NUMBER options")]
pub(super) struct OptionsError;

impl Default for NumericOptions {
    fn default() -> Self {
        Self {
            fluent: FluentNumberOptions::default(),
            notation: Notation::Ordinary,
        }
    }
}

impl NumericOptions {
    pub(super) fn merged(&self, named: &FluentArgs<'_>) -> Result<Self, OptionsError> {
        for (key, value) in named.iter() {
            if !valid(key, value) {
                return Err(OptionsError);
            }
        }
        let mut next = self.clone();
        next.fluent.merge(named);
        if let Some(FluentValue::String(value)) = named.get("notation") {
            next.notation = match value.as_ref() {
                "compact" => Notation::Compact,
                _ => Notation::Ordinary,
            };
        }
        // An inner NUMBER rounds before an outer NUMBER. Merge and prepare
        // from that numeric result, rather than recovering a number from text.
        if let Some(maximum) = next.fluent.maximum_fraction_digits {
            next.fluent.minimum_fraction_digits = Some(
                next.fluent
                    .minimum_fraction_digits
                    .unwrap_or(0)
                    .min(maximum),
            );
        }
        Ok(next)
    }
}

fn valid(key: &str, value: &FluentValue<'_>) -> bool {
    match (key, value) {
        ("useGrouping", FluentValue::String(value)) => matches!(value.as_ref(), "true" | "false"),
        ("type", FluentValue::String(value)) => matches!(value.as_ref(), "cardinal" | "ordinal"),
        ("style", FluentValue::String(value)) => matches!(value.as_ref(), "decimal" | "currency"),
        ("currency", FluentValue::String(value)) => value == "USD",
        ("currencyDisplay", FluentValue::String(value)) => value == "symbol",
        ("notation", FluentValue::String(value)) => {
            matches!(value.as_ref(), "standard" | "compact")
        }
        (
            "minimumIntegerDigits" | "minimumFractionDigits" | "maximumFractionDigits",
            FluentValue::Number(value),
        ) => {
            value.value.is_finite()
                && value.value >= 0.0
                && value.value <= f64::from(MAX_MESSAGE_DIGITS)
                && value.value.fract().classify() == std::num::FpCategory::Zero
        }
        _ => false,
    }
}
