// Experimental replacement for the Number block in the scratch View overlay.
// It is not included or exported by the product worktree.

/// Numeric input kept exact until this translator prepares its two namespaces.
#[derive(Debug, Clone, PartialEq)]
pub struct Number(NumberInput);

#[derive(Debug, Clone)]
enum NumberInput {
    Count(u64),
    Decimal(
        fluent_bundle::types::FluentNumber,
        crate::options_probe::Notation,
    ),
}

impl PartialEq for NumberInput {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Count(left), Self::Count(right)) => left == right,
            (Self::Decimal(left, left_notation), Self::Decimal(right, right_notation)) => {
                left.value.to_bits() == right.value.to_bits()
                    && left.options == right.options
                    && left_notation == right_notation
            }
            (Self::Count(_), Self::Decimal(..)) | (Self::Decimal(..), Self::Count(_)) => false,
        }
    }
}

/// The legacy decimal constructor's compatibility bound, retained by the probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("message decimals must be finite, within u32 magnitude, and at most nine decimal places")]
pub struct NumberError;

impl Number {
    /// Preserve the existing source API while widening its private carrier.
    #[must_use]
    pub fn count(value: u32) -> Self {
        Self::unsigned(u64::from(value))
    }

    /// Experimental exact unsigned entry, with no conversion to floating point.
    #[must_use]
    pub const fn unsigned(value: u64) -> Self {
        Self(NumberInput::Count(value))
    }

    /// Preserve the existing bounded decimal API during this cost experiment.
    /// # Errors
    /// Rejects values outside the legacy constructor's documented contract.
    pub fn decimal(value: f64) -> Result<Self, NumberError> {
        if !value.is_finite()
            || value.abs() > f64::from(u32::MAX)
            || value
                .to_string()
                .split_once('.')
                .is_some_and(|(_, fraction)| fraction.len() > 9)
        {
            return Err(NumberError);
        }
        Ok(Self(NumberInput::Decimal(
            value.into(),
            crate::options_probe::Notation::Ordinary,
        )))
    }

    /// Experimental full-finite amount and closed policy; no magnitude ceiling.
    #[must_use]
    pub fn amount(value: vessel_proto::conversation::FiniteNumber, unit: AmountUnit) -> Self {
        use crate::options_probe::Notation;
        use fluent_bundle::types::{FluentNumber, FluentNumberOptions, FluentNumberStyle};
        const AMOUNT_FRACTION_DIGITS: usize = 2;
        const COMPACT_FRACTION_DIGITS: usize = 1;
        let mut options = FluentNumberOptions::default();
        let notation = match unit {
            AmountUnit::Usd => {
                options.style = FluentNumberStyle::Currency;
                options.currency = Some("USD".to_owned());
                options.minimum_fraction_digits = Some(AMOUNT_FRACTION_DIGITS);
                options.maximum_fraction_digits = Some(AMOUNT_FRACTION_DIGITS);
                Notation::Ordinary
            }
            AmountUnit::CompactTokens => {
                options.maximum_fraction_digits = Some(COMPACT_FRACTION_DIGITS);
                Notation::Compact
            }
            AmountUnit::Quantity => {
                options.maximum_fraction_digits = Some(AMOUNT_FRACTION_DIGITS);
                Notation::Ordinary
            }
        };
        Self(NumberInput::Decimal(
            FluentNumber::new(value.get(), options),
            notation,
        ))
    }

    fn fluent(
        self,
        context: &std::sync::Arc<crate::carrier_probe::NumberContext>,
    ) -> Result<fluent_bundle::FluentValue<'static>, FormatError> {
        use crate::options_probe::NumericOptions;
        let (value, options) = match self.0 {
            NumberInput::Count(value) => (
                fixed_decimal::Decimal::from(value),
                NumericOptions::default(),
            ),
            NumberInput::Decimal(value, notation) => {
                let decimal = fixed_decimal::Decimal::try_from_f64(
                    value.value,
                    fixed_decimal::FloatPrecision::RoundTrip,
                )
                .map_err(|error| {
                    FormatError::Numeric(crate::kernel_probe::KernelError::from(error).into())
                })?;
                (
                    decimal,
                    NumericOptions {
                        fluent: value.options,
                        notation,
                    },
                )
            }
        };
        context
            .bind(value, options)
            .map_err(|error| FormatError::Numeric(error.into()))
    }
}

/// Retained numeric diagnostic without exposing the experimental backend API.
#[derive(Debug, Clone)]
pub struct NumericFailure(std::sync::Arc<crate::kernel_probe::KernelError>);

impl std::fmt::Display for NumericFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, formatter)
    }
}

impl std::error::Error for NumericFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

impl From<crate::kernel_probe::KernelError> for NumericFailure {
    fn from(error: crate::kernel_probe::KernelError) -> Self {
        Self(std::sync::Arc::new(error))
    }
}

/// Closed presentation policies for the scratch amount entry, not wire units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountUnit {
    /// Whole USD currency pattern with two fraction digits.
    Usd,
    /// Short compact token amount with at most one fraction digit.
    CompactTokens,
    /// Ordinary quantity with at most two fraction digits.
    Quantity,
}
