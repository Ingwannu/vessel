//! Single-locale narrow candidate, still unlinked and uncompiled.
//! A later integration supplies whole English fallback and preparation caches.

#[cfg(test)]
use super::{
    backend_contract_probe::{CompactView, NumericBackend},
    decimal_probe::{self, DecimalPresentation, ExactCount},
    pattern_probe::NumberParts,
    placement_probe::Grouping,
};
use fixed_decimal::{Decimal, Sign};
use icu_locale_core::Locale;
#[cfg(test)]
use icu_plurals::PluralCategory;
use icu_plurals::PluralRules;
#[cfg(test)]
use vessel_proto::conversation::FiniteNumber;

use super::{
    backend_probe::Backend,
    pattern_probe::{NumberSign, PatternError, Patterns},
    placement_probe::{PlacementError, Symbols},
};

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AmountStyle {
    Usd,
    CompactTokens,
    Quantity,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(super) struct Amount {
    value: FiniteNumber,
    style: AmountStyle,
}

#[cfg(test)]
impl Amount {
    pub(super) const fn new(value: FiniteNumber, style: AmountStyle) -> Self {
        Self { value, style }
    }
}

// FiniteNumber's numeric Eq merges signed zeros. A presentation key must not:
// source Intl renders their signs differently even after rounding to zero.
#[cfg(test)]
impl PartialEq for Amount {
    fn eq(&self, other: &Self) -> bool {
        self.style == other.style && self.value.get().to_bits() == other.value.get().to_bits()
    }
}

#[cfg(test)]
impl Eq for Amount {}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PluralMode {
    Cardinal,
    Ordinal,
}

#[cfg(test)]
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
    #[error("compact plural exponent: {0}")]
    PluralExponent(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    Backend(super::backend_probe::BackendError),
    #[error(transparent)]
    Placement(#[from] PlacementError),
    #[error(transparent)]
    Pattern(#[from] PatternError),
    #[error(
        "numeric preparation failed in the selected locale ({selected}) and English ({english})"
    )]
    Fallback {
        selected: Box<Self>,
        #[source]
        english: Box<Self>,
    },
}

impl From<fixed_decimal::LimitError> for KernelError {
    fn from(error: fixed_decimal::LimitError) -> Self {
        Self::Decimal(error)
    }
}

#[derive(Debug)]
pub(super) struct NumberKernel<'a> {
    backend: Backend<'a>,
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
            backend: Backend::try_from((locale.clone(), symbols))?,
            patterns: Patterns::new(tag, patterns)?,
            cardinal: PluralRules::try_new_cardinal(locale.clone().into())?,
            ordinal: PluralRules::try_new_ordinal(locale.into())?,
        })
    }

    pub(super) fn require_complete(&self) -> Result<(), KernelError> {
        Ok(self.patterns.require_complete()?)
    }

    pub(super) fn category(
        &self,
        value: &Decimal,
        mode: fluent_bundle::types::FluentNumberType,
        exponent: Option<super::decimal_probe::CompactExponent>,
    ) -> Result<icu_plurals::PluralCategory, KernelError> {
        let exponent = exponent
            .map(|value| u8::try_from(value.get()))
            .transpose()?
            .unwrap_or_default();
        // Compact CLDR rules need the exponent as well as the significand.
        // The maintained engine derives the operands, without parsing digits.
        let operands =
            icu_plurals::PluralOperands::from_significand_and_exponent(&value.absolute, exponent);
        Ok(match mode {
            fluent_bundle::types::FluentNumberType::Cardinal => {
                self.cardinal.category_for(operands)
            }
            fluent_bundle::types::FluentNumberType::Ordinal => self.ordinal.category_for(operands),
        })
    }

    #[cfg(test)]
    pub(super) fn amount(&self, amount: Amount) -> Result<String, KernelError> {
        match amount.style {
            AmountStyle::Quantity => self.decimal(amount.value, DecimalPresentation::Quantity),
            AmountStyle::Usd => self.decimal(amount.value, DecimalPresentation::Usd),
            AmountStyle::CompactTokens => self.compact(amount.value),
        }
    }

    #[cfg(test)]
    fn decimal(
        &self,
        input: FiniteNumber,
        presentation: DecimalPresentation,
    ) -> Result<String, KernelError> {
        let value = decimal_probe::rounded(input, presentation)?;
        let absolute = self.backend.absolute(&value, Grouping::Auto)?;
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

    #[cfg(test)]
    pub(super) fn count(
        &self,
        value: ExactCount,
        mode: PluralMode,
    ) -> Result<CountDisplay, KernelError> {
        let value_count = value.get();
        let value = value.decimal();
        let digits = self.backend.absolute(&value, Grouping::Auto)?;
        let rules = match mode {
            PluralMode::Cardinal => &self.cardinal,
            PluralMode::Ordinal => &self.ordinal,
        };
        Ok(CountDisplay {
            digits,
            category: rules.category_for(value_count),
        })
    }

    #[cfg(test)]
    fn compact(&self, value: FiniteNumber) -> Result<String, KernelError> {
        let prepared = self
            .backend
            .compact(&decimal_probe::decimal(value)?, &self.patterns)?;
        self.backend.compact_display(
            CompactView {
                value: prepared.value(),
                exponent: prepared.exponent(),
                grouping: Grouping::MinimumTwo,
            },
            &self.patterns,
            &self.cardinal,
        )
    }
}

pub(super) fn number_sign(value: &Decimal) -> NumberSign {
    match value.sign() {
        Sign::Negative => NumberSign::Negative,
        Sign::None | Sign::Positive => NumberSign::Positive,
    }
}

#[cfg(test)]
#[path = "kernel_probe_tests.rs"]
mod tests;

#[path = "kernel_prepare_probe.rs"]
mod prepare;

pub(super) use prepare::{PreparedNumeric, prepare_options};
