//! ADR 0101 arithmetic candidate, not linked into view or the app.
//! The controlled offline experiment must compile/test this before any adoption.
//! Locale patterns/grouping and the Fluent adapter are separate measurements;
//! this module does not claim to be a complete formatter.

use fixed_decimal::{
    Decimal, FloatPrecision, LimitError, SignedRoundingMode, UnsignedRoundingMode,
};
use vessel_proto::conversation::FiniteNumber;

const TWO_FRACTION_MAGNITUDE: i16 = -2;
const COMPACT_FRACTION_MAGNITUDE: i16 = -1;
const MIN_COMPACT_MAGNITUDE: i16 = 1;
const MAX_FINITE_MAGNITUDE: i16 = 308;
const HALF_EXPAND: SignedRoundingMode =
    SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfExpand);

/// Preserve the source's numeric input, including signed zero. The backend
/// documents finite input; a backend error here is not a u32 range policy.
pub(super) fn decimal(value: FiniteNumber) -> Result<Decimal, LimitError> {
    Decimal::try_from_f64(value.get(), FloatPrecision::RoundTrip)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DecimalPresentation {
    Usd,
    Quantity,
}

pub(super) fn rounded(
    value: FiniteNumber,
    presentation: DecimalPresentation,
) -> Result<Decimal, LimitError> {
    let mut value = decimal(value)?;
    value.round_with_mode(TWO_FRACTION_MAGNITUDE, HALF_EXPAND);
    match presentation {
        DecimalPresentation::Usd => value.pad_end(TWO_FRACTION_MAGNITUDE),
        DecimalPresentation::Quantity => value.trim_end(),
    }
    Ok(value)
}

/// This capability never converts through the finite-float carrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExactCount(u64);

impl ExactCount {
    pub(super) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(super) const fn get(self) -> u64 {
        self.0
    }

    pub(super) fn decimal(self) -> Decimal {
        Decimal::from(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CompactExponent(i16);

impl CompactExponent {
    pub(super) const fn get(self) -> i16 {
        self.0
    }
}

// ICU's compact API returns u8: every possible backend value is inside the
// finite magnitude bound, without a fallible or truncating integer cast.
impl From<u8> for CompactExponent {
    fn from(value: u8) -> Self {
        Self(i16::from(value))
    }
}

pub(super) trait ScaleSource {
    fn exponent(&self, magnitude: i16) -> CompactExponent;
}

/// Numeric scale data is injected, never selected by a language branch here.
/// The eventual folder loader must apply the same validation before caching.
#[derive(Debug)]
pub(super) struct CompactScales<'a>(&'a [i16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum ScaleError {
    #[error("compact scale is outside the finite amount magnitude range")]
    OutOfRange,
    #[error("compact scales must be strictly increasing")]
    NotIncreasing,
}

impl<'a> CompactScales<'a> {
    pub(super) fn new(values: &'a [i16]) -> Result<Self, ScaleError> {
        if values
            .iter()
            .any(|value| !(MIN_COMPACT_MAGNITUDE..=MAX_FINITE_MAGNITUDE).contains(value))
        {
            return Err(ScaleError::OutOfRange);
        }
        if values
            .iter()
            .zip(values.iter().skip(1))
            .any(|(a, b)| a >= b)
        {
            return Err(ScaleError::NotIncreasing);
        }
        Ok(Self(values))
    }
}

impl ScaleSource for CompactScales<'_> {
    fn exponent(&self, magnitude: i16) -> CompactExponent {
        CompactExponent(
            self.0
                .iter()
                .rev()
                .find(|value| **value <= magnitude)
                .copied()
                .unwrap_or_default(),
        )
    }
}

#[derive(Debug)]
pub(super) struct CompactQuantity {
    value: Decimal,
    exponent: CompactExponent,
}

impl CompactQuantity {
    pub(super) const fn value(&self) -> &Decimal {
        &self.value
    }

    pub(super) const fn exponent(&self) -> CompactExponent {
        self.exponent
    }
}

pub(super) fn compact(
    value: FiniteNumber,
    scales: &impl ScaleSource,
) -> Result<CompactQuantity, LimitError> {
    compact_decimal(&decimal(value)?, scales)
}

pub(super) fn compact_decimal(
    original: &Decimal,
    scales: &impl ScaleSource,
) -> Result<CompactQuantity, LimitError> {
    let exponent = scales.exponent(original.nonzero_magnitude_start());
    let rounded = scaled(original, exponent)?;
    let magnitude = rounded
        .nonzero_magnitude_start()
        .checked_add(exponent.get())
        .ok_or(LimitError)?;
    let carried = scales.exponent(magnitude);
    // Fraction rounding can raise the magnitude by at most one. Re-select the
    // scale from that result, then round the ORIGINAL value: rounding the
    // first rounded coefficient again could introduce double-rounding error.
    let value = if carried == exponent {
        rounded
    } else {
        scaled(original, carried)?
    };
    Ok(CompactQuantity {
        value,
        exponent: carried,
    })
}

fn scaled(value: &Decimal, exponent: CompactExponent) -> Result<Decimal, LimitError> {
    let mut value = value.clone();
    value.multiply_pow10(exponent.get().checked_neg().ok_or(LimitError)?);
    value.round_with_mode(COMPACT_FRACTION_MAGNITUDE, HALF_EXPAND);
    value.trim_end();
    Ok(value)
}

#[cfg(test)]
#[path = "oracle_probe.rs"]
pub(super) mod oracle;

#[cfg(test)]
#[path = "decimal_probe_tests.rs"]
mod tests;
