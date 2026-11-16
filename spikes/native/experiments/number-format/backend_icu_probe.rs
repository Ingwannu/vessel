//! Alternative measured backend: ICU owns digits/grouping and compact data.
//! This is not adopted; full compiled data and the unstable API are measured.

use fixed_decimal::{Decimal, Sign};
use icu_decimal::{CompactDecimalFormatter, DecimalFormatter, options::GroupingStrategy};
use icu_locale_core::Locale;
use std::marker::PhantomData;

use super::{
    backend_contract_probe::{CompactView, NumericBackend},
    decimal_probe::{self, CompactExponent, CompactQuantity, ScaleSource},
    kernel_probe::KernelError,
    pattern_probe::Patterns,
    placement_probe::{Grouping, Symbols},
};

#[derive(Debug, thiserror::Error)]
pub(super) enum BackendError {
    #[error(transparent)]
    Compact(#[from] icu_decimal::error::CompactExponentError),
    #[error("compact exponent: {0}")]
    Exponent(#[from] std::num::TryFromIntError),
}

#[derive(Debug)]
pub(super) struct Backend<'a> {
    auto: DecimalFormatter,
    never: DecimalFormatter,
    minimum_two: DecimalFormatter,
    compact: CompactDecimalFormatter,
    compact_never: CompactDecimalFormatter,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> TryFrom<(Locale, Symbols<'a>)> for Backend<'a> {
    type Error = KernelError;
    fn try_from((locale, _symbols): (Locale, Symbols<'a>)) -> Result<Self, Self::Error> {
        Ok(Self {
            auto: DecimalFormatter::try_new(locale.clone().into(), GroupingStrategy::Auto.into())?,
            never: DecimalFormatter::try_new(
                locale.clone().into(),
                GroupingStrategy::Never.into(),
            )?,
            minimum_two: DecimalFormatter::try_new(
                locale.clone().into(),
                GroupingStrategy::Min2.into(),
            )?,
            compact: CompactDecimalFormatter::try_new_short(
                locale.clone().into(),
                GroupingStrategy::Min2.into(),
            )?,
            compact_never: CompactDecimalFormatter::try_new_short(
                locale.into(),
                GroupingStrategy::Never.into(),
            )?,
            lifetime: PhantomData,
        })
    }
}

impl NumericBackend for Backend<'_> {
    fn absolute(&self, value: &Decimal, grouping: Grouping) -> Result<String, KernelError> {
        let formatter = match grouping {
            Grouping::Auto => &self.auto,
            Grouping::Never => &self.never,
            Grouping::MinimumTwo => &self.minimum_two,
        };
        // Supply an unsigned value to the common whole USD/sign pattern;
        // never remove a sign by parsing or trimming the localized output.
        let mut value = value.clone();
        value.sign = Sign::None;
        Ok(formatter.format(&value).to_string())
    }

    fn compact(
        &self,
        value: &Decimal,
        patterns: &Patterns,
    ) -> Result<CompactQuantity, KernelError> {
        // Both candidates require a source-folder compact contract; if absent,
        // the outer numeric fallback redoes the whole English presentation.
        let _folder_contract = patterns.scales()?;
        Ok(decimal_probe::compact_decimal(value, &self.compact)?)
    }

    fn compact_display(
        &self,
        view: CompactView<'_>,
        _patterns: &Patterns,
        _cardinal: &icu_plurals::PluralRules,
    ) -> Result<String, KernelError> {
        let formatter = match view.grouping {
            Grouping::Never => &self.compact_never,
            Grouping::Auto | Grouping::MinimumTwo => &self.compact,
        };
        let exponent = u8::try_from(view.exponent.get())
            .map_err(|error| KernelError::Backend(BackendError::from(error)))?;
        formatter
            .format_with_exponent(view.value, exponent)
            .map(|value| value.to_string())
            .map_err(|error| KernelError::Backend(BackendError::from(error)))
    }
}

impl ScaleSource for CompactDecimalFormatter {
    fn exponent(&self, magnitude: i16) -> CompactExponent {
        self.compact_exponent_for_magnitude(magnitude).into()
    }
}
