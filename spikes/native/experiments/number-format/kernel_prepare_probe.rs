//! Numeric preparation shared by direct arguments and the NUMBER callback.
//! Both the rendered value and selector operands are derived from one decimal.

use fixed_decimal::{Decimal, SignedRoundingMode, UnsignedRoundingMode};
use fluent_bundle::types::FluentNumberStyle;
use icu_plurals::PluralCategory;

use super::super::{
    backend_contract_probe::{CompactView, NumericBackend},
    options_probe::{Notation, NumericOptions},
    pattern_probe::NumberParts,
    placement_probe::Grouping,
};
use super::{KernelError, NumberKernel, number_sign};

const HALF_EXPAND: SignedRoundingMode =
    SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfExpand);
const USD_FRACTION: i16 = -2;

#[derive(Debug, Clone, PartialEq)]
pub(in super::super) struct PreparedNumeric {
    pub(in super::super) decimal: Decimal,
    pub(in super::super) operand: Decimal,
    pub(in super::super) exponent: Option<super::super::decimal_probe::CompactExponent>,
    pub(in super::super) display: String,
    pub(in super::super) category: PluralCategory,
}

pub(in super::super) fn prepare_options(
    kernel: &NumberKernel<'_>,
    input: &Decimal,
    options: &NumericOptions,
) -> Result<PreparedNumeric, KernelError> {
    let mut value = input.clone();
    let compact = match options.notation {
        Notation::Ordinary => None,
        Notation::Compact => {
            let prepared = kernel.backend.compact(&value, &kernel.patterns)?;
            value = prepared.value().clone();
            Some(prepared.exponent())
        }
    };
    let fluent = &options.fluent;
    if let Some(maximum) = fluent.maximum_fraction_digits {
        let maximum = i16::try_from(maximum).map_err(|_| fixed_decimal::LimitError)?;
        value.round_with_mode(-maximum, HALF_EXPAND);
    } else if fluent.style == FluentNumberStyle::Currency {
        value.round_with_mode(USD_FRACTION, HALF_EXPAND);
    }
    value.trim_end();
    if let Some(minimum) = fluent.minimum_fraction_digits {
        let minimum = i16::try_from(minimum).map_err(|_| fixed_decimal::LimitError)?;
        value.pad_end(-minimum);
    } else if fluent.style == FluentNumberStyle::Currency {
        value.pad_end(USD_FRACTION);
    }
    if let Some(minimum) = fluent.minimum_integer_digits {
        let minimum = i16::try_from(minimum).map_err(|_| fixed_decimal::LimitError)?;
        value.pad_start(minimum);
    }
    let grouping = if !fluent.use_grouping {
        Grouping::Never
    } else if compact.is_some() {
        Grouping::MinimumTwo
    } else {
        Grouping::Auto
    };
    let category = kernel.category(&value, fluent.r#type, compact)?;
    let sign = number_sign(&value);
    let display = if let Some(exponent) = compact {
        kernel.backend.compact_display(
            CompactView {
                value: &value,
                exponent,
                grouping,
            },
            &kernel.patterns,
            &kernel.cardinal,
        )?
    } else {
        let absolute = kernel.backend.absolute(&value, grouping)?;
        let parts = if fluent.style == FluentNumberStyle::Currency {
            NumberParts::Usd {
                absolute: &absolute,
                sign,
            }
        } else {
            NumberParts::Quantity {
                absolute: &absolute,
                sign,
            }
        };
        kernel.patterns.render(parts)?
    };
    let operand = value.clone();
    // Keep the amount, not its compact coefficient, as the next NUMBER's
    // operand. Otherwise nested formatting would turn 10K into 10.
    if let Some(exponent) = compact {
        value.multiply_pow10(exponent.get());
    }
    Ok(PreparedNumeric {
        display,
        decimal: value,
        operand,
        exponent: compact,
        category,
    })
}
