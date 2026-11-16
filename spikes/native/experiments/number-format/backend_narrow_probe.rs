//! Folder-owned placement and compact patterns; shares maintained arithmetic.

use fixed_decimal::Decimal;
use icu_locale_core::Locale;

use super::{
    backend_contract_probe::{CompactView, NumericBackend},
    decimal_probe::{self, CompactQuantity},
    kernel_probe::{KernelError, number_sign},
    pattern_probe::{NumberParts, Patterns},
    placement_probe::{Grouping, PlacementError, Symbols},
};

pub(super) type BackendError = PlacementError;

#[derive(Debug)]
pub(super) struct Backend<'a>(Symbols<'a>);

impl<'a> TryFrom<(Locale, Symbols<'a>)> for Backend<'a> {
    type Error = KernelError;
    fn try_from((_locale, symbols): (Locale, Symbols<'a>)) -> Result<Self, Self::Error> {
        Ok(Self(symbols))
    }
}

impl NumericBackend for Backend<'_> {
    fn absolute(&self, value: &Decimal, grouping: Grouping) -> Result<String, KernelError> {
        self.0
            .format(&value.absolute, grouping)
            .map_err(KernelError::Backend)
    }

    fn compact(
        &self,
        value: &Decimal,
        patterns: &Patterns,
    ) -> Result<CompactQuantity, KernelError> {
        Ok(decimal_probe::compact_decimal(value, &patterns.scales()?)?)
    }

    fn compact_display(
        &self,
        view: CompactView<'_>,
        patterns: &Patterns,
        cardinal: &icu_plurals::PluralRules,
    ) -> Result<String, KernelError> {
        let absolute = self.absolute(view.value, view.grouping)?;
        Ok(patterns.render(NumberParts::Compact {
            absolute: &absolute,
            sign: number_sign(view.value),
            exponent: view.exponent,
            category: cardinal.category_for(
                icu_plurals::PluralOperands::from_significand_and_exponent(
                    &view.value.absolute,
                    u8::try_from(view.exponent.get())?,
                ),
            ),
        })?)
    }
}
