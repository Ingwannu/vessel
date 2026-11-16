//! A shared experiment interface; each immutable overlay chooses one backend.
//! Arithmetic/options/carrier/fallback remain identical in both measurements.

use fixed_decimal::Decimal;
use icu_plurals::PluralRules;

use super::{
    decimal_probe::{CompactExponent, CompactQuantity},
    kernel_probe::KernelError,
    pattern_probe::Patterns,
    placement_probe::Grouping,
};

pub(super) struct CompactView<'a> {
    pub(super) value: &'a Decimal,
    pub(super) exponent: CompactExponent,
    pub(super) grouping: Grouping,
}

pub(super) trait NumericBackend {
    fn absolute(&self, value: &Decimal, grouping: Grouping) -> Result<String, KernelError>;
    fn compact(&self, value: &Decimal, patterns: &Patterns)
    -> Result<CompactQuantity, KernelError>;
    fn compact_display(
        &self,
        view: CompactView<'_>,
        patterns: &Patterns,
        cardinal: &PluralRules,
    ) -> Result<String, KernelError>;
}
