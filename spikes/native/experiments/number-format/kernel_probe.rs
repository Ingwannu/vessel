//! Single-locale narrow candidate, still unlinked and uncompiled.
//! A later integration supplies whole English fallback and preparation caches.

use fixed_decimal::{Decimal, Sign};
use icu_locale_core::Locale;
use icu_plurals::{PluralCategory, PluralRules};
use vessel_proto::conversation::FiniteNumber;

use super::{
    decimal_probe::{self, DecimalPresentation, ExactCount},
    pattern_probe::{NumberParts, NumberSign, PatternError, Patterns},
    placement_probe::{Grouping, PlacementError, Symbols},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AmountStyle {
    Usd,
    CompactTokens,
    Quantity,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Amount {
    value: FiniteNumber,
    style: AmountStyle,
}

impl Amount {
    pub(super) const fn new(value: FiniteNumber, style: AmountStyle) -> Self {
        Self { value, style }
    }
}

// FiniteNumber's numeric Eq merges signed zeros. A presentation key must not:
// source Intl renders their signs differently even after rounding to zero.
impl PartialEq for Amount {
    fn eq(&self, other: &Self) -> bool {
        self.style == other.style && self.value.get().to_bits() == other.value.get().to_bits()
    }
}

impl Eq for Amount {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PluralMode {
    Cardinal,
    Ordinal,
}

#[derive(Debug)]
pub(super) struct CountDisplay {
    pub(super) digits: String,
    pub(super) category: PluralCategory,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum KernelError {
    #[error("locale parser: {0}")]
    Locale(#[from] icu_locale_core::ParseError),
    #[error("plural data: {0}")]
    Data(#[from] icu_provider::DataError),
    #[error("decimal backend: {0}")]
    Decimal(fixed_decimal::LimitError),
    #[error(transparent)]
    Placement(#[from] PlacementError),
    #[error(transparent)]
    Pattern(#[from] PatternError),
}

impl From<fixed_decimal::LimitError> for KernelError {
    fn from(error: fixed_decimal::LimitError) -> Self {
        Self::Decimal(error)
    }
}

#[derive(Debug)]
pub(super) struct NumberKernel<'a> {
    symbols: Symbols<'a>,
    patterns: Patterns,
    cardinal: PluralRules,
    ordinal: PluralRules,
}

impl<'a> NumberKernel<'a> {
    pub(super) fn new(
        tag: &str,
        patterns: &str,
        symbols: Symbols<'a>,
    ) -> Result<Self, KernelError> {
        let locale: Locale = tag.parse()?;
        Ok(Self {
            symbols,
            patterns: Patterns::new(tag, patterns)?,
            cardinal: PluralRules::try_new_cardinal(locale.clone().into())?,
            ordinal: PluralRules::try_new_ordinal(locale.into())?,
        })
    }

    pub(super) fn require_complete(&self) -> Result<(), KernelError> {
        Ok(self.patterns.require_complete()?)
    }

    pub(super) fn amount(&self, amount: Amount) -> Result<String, KernelError> {
        match amount.style {
            AmountStyle::Quantity => self.decimal(amount.value, DecimalPresentation::Quantity),
            AmountStyle::Usd => self.decimal(amount.value, DecimalPresentation::Usd),
            AmountStyle::CompactTokens => self.compact(amount.value),
        }
    }

    fn decimal(
        &self,
        input: FiniteNumber,
        presentation: DecimalPresentation,
    ) -> Result<String, KernelError> {
        let value = decimal_probe::rounded(input, presentation)?;
        let absolute = self.symbols.format(&value.absolute, Grouping::Auto)?;
        let sign = number_sign(&value);
        let parts = match presentation {
            DecimalPresentation::Usd => NumberParts::Usd {
                absolute: &absolute,
                sign,
            },
            DecimalPresentation::Quantity => NumberParts::Quantity {
                absolute: &absolute,
                sign,
            },
        };
        Ok(self.patterns.render(parts)?)
    }

    pub(super) fn count(
        &self,
        value: ExactCount,
        mode: PluralMode,
    ) -> Result<CountDisplay, KernelError> {
        let value = value.decimal();
        let digits = self.symbols.format(&value.absolute, Grouping::Auto)?;
        let rules = match mode {
            PluralMode::Cardinal => &self.cardinal,
            PluralMode::Ordinal => &self.ordinal,
        };
        Ok(CountDisplay {
            digits,
            category: rules.category_for(&value),
        })
    }

    fn compact(&self, value: FiniteNumber) -> Result<String, KernelError> {
        let scales = self.patterns.scales()?;
        let prepared = decimal_probe::compact(value, &scales)?;
        let absolute = self
            .symbols
            .format(&prepared.value().absolute, Grouping::MinimumTwo)?;
        Ok(self.patterns.render(NumberParts::Compact {
            absolute: &absolute,
            sign: number_sign(prepared.value()),
            exponent: prepared.exponent(),
            category: self.cardinal.category_for(prepared.value()),
        })?)
    }
}

fn number_sign(value: &Decimal) -> NumberSign {
    match value.sign() {
        Sign::Negative => NumberSign::Negative,
        Sign::None | Sign::Positive => NumberSign::Positive,
    }
}

#[cfg(test)]
#[path = "kernel_probe_tests.rs"]
mod tests;
