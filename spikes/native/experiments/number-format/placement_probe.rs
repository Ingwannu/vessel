//! Unlinked narrow candidate: digit placement over a maintained decimal.
//! It parses/rounds no number and selects no language. The caller supplies
//! validated folder data and an unsigned magnitude; whole sign/unit patterns
//! remain outside this layer.

use std::{collections::BTreeSet, num::NonZeroUsize};

use fixed_decimal::UnsignedDecimal;

const RADIX: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Grouping {
    Never,
    Auto,
    MinimumTwo,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum PlacementError {
    #[error("numbering data needs ten unique digit characters: {0:?}")]
    Digits(Vec<char>),
    #[error("decimal backend returned an out-of-range digit: {0}")]
    Digit(u8),
}

#[derive(Debug)]
pub(super) struct Symbols<'a> {
    group: &'a str,
    decimal: &'a str,
    digits: [char; RADIX],
    primary: NonZeroUsize,
    secondary: NonZeroUsize,
}

impl<'a> Symbols<'a> {
    pub(super) fn new(
        group: &'a str,
        decimal: &'a str,
        digits: &str,
        primary: NonZeroUsize,
        secondary: NonZeroUsize,
    ) -> Result<Self, PlacementError> {
        let digits: Vec<_> = digits.chars().collect();
        if digits.iter().copied().collect::<BTreeSet<_>>().len() != RADIX {
            return Err(PlacementError::Digits(digits));
        }
        let digits = digits.try_into().map_err(PlacementError::Digits)?;
        Ok(Self {
            group,
            decimal,
            digits,
            primary,
            secondary,
        })
    }

    pub(super) fn format(
        &self,
        value: &UnsignedDecimal,
        grouping: Grouping,
    ) -> Result<String, PlacementError> {
        let range = value.magnitude_range();
        let upper = (*range.end()).max(0);
        let lower = (*range.start()).min(0);
        // An i16 non-negative magnitude plus one fits even a 16-bit usize.
        // Saturating operations express the already-proved arithmetic bound;
        // they never clamp the numeric input or remove any of its digits.
        let length = usize::from(upper.unsigned_abs()).saturating_add(1);
        let grouped = match grouping {
            Grouping::Never => false,
            Grouping::Auto => length > self.primary.get(),
            Grouping::MinimumTwo => length > self.primary.get().saturating_add(1),
        };
        let mut output = String::new();
        for magnitude in (0..=upper).rev() {
            let remaining = usize::from(magnitude.unsigned_abs()).saturating_add(1);
            if grouped
                && magnitude != upper
                && remaining >= self.primary.get()
                && remaining
                    .saturating_sub(self.primary.get())
                    .is_multiple_of(self.secondary.get())
            {
                output.push_str(self.group);
            }
            output.push(self.digit(value.digit_at(magnitude))?);
        }
        if lower < 0 {
            output.push_str(self.decimal);
            for magnitude in (lower..0).rev() {
                output.push(self.digit(value.digit_at(magnitude))?);
            }
        }
        Ok(output)
    }

    fn digit(&self, value: u8) -> Result<char, PlacementError> {
        self.digits
            .get(usize::from(value))
            .copied()
            .ok_or(PlacementError::Digit(value))
    }
}

#[cfg(test)]
#[path = "placement_probe_tests.rs"]
mod tests;
