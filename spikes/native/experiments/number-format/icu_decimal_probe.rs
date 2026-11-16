//! Unlinked ICU comparison candidate. Arithmetic is shared with the narrow
//! candidate; ICU supplies locale grouping, digit placement and compact data.
//! Currency symbols/patterns remain a separate common folder-owned layer.

use super::decimal_probe::{self, CompactExponent, DecimalPresentation, ExactCount, ScaleSource};
use icu_decimal::{CompactDecimalFormatter, DecimalFormatter, error::CompactExponentError};
use icu_locale_core::Locale;
use vessel_proto::conversation::FiniteNumber;

#[derive(Debug)]
pub(super) struct IcuNumbers {
    decimal: DecimalFormatter,
    compact: CompactDecimalFormatter,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum ProbeError {
    #[error("decimal backend: {0}")]
    Decimal(fixed_decimal::LimitError),
    #[error("compact backend: {0}")]
    Compact(#[from] CompactExponentError),
    #[error("ICU returned an exponent outside its u8 contract: {value}: {source}")]
    Exponent {
        value: i16,
        source: std::num::TryFromIntError,
    },
}

// This version's LimitError has Display but no Error implementation. Retain
// the typed diagnostic without asking thiserror for an unsupported source.
impl From<fixed_decimal::LimitError> for ProbeError {
    fn from(error: fixed_decimal::LimitError) -> Self {
        Self::Decimal(error)
    }
}

impl IcuNumbers {
    pub(super) fn new(locale: Locale) -> Result<Self, icu_provider::DataError> {
        Ok(Self {
            decimal: DecimalFormatter::try_new(locale.clone().into(), Default::default())?,
            compact: CompactDecimalFormatter::try_new_short(locale.into(), Default::default())?,
        })
    }

    pub(super) fn amount(
        &self,
        value: FiniteNumber,
        presentation: DecimalPresentation,
    ) -> Result<String, ProbeError> {
        let value = decimal_probe::rounded(value, presentation)?;
        Ok(self.decimal.format(&value).to_string())
    }

    pub(super) fn count(&self, value: ExactCount) -> String {
        let value = value.decimal();
        self.decimal.format(&value).to_string()
    }

    pub(super) fn compact(&self, value: FiniteNumber) -> Result<String, ProbeError> {
        let prepared = decimal_probe::compact(value, &self.compact)?;
        let exponent = prepared.exponent().get();
        let exponent = u8::try_from(exponent).map_err(|source| ProbeError::Exponent {
            value: exponent,
            source,
        })?;
        Ok(self
            .compact
            .format_with_exponent(prepared.value(), exponent)?
            .to_string())
    }
}

impl ScaleSource for CompactDecimalFormatter {
    fn exponent(&self, magnitude: i16) -> CompactExponent {
        self.compact_exponent_for_magnitude(magnitude).into()
    }
}

#[cfg(test)]
#[path = "icu_decimal_probe_tests.rs"]
mod tests;
