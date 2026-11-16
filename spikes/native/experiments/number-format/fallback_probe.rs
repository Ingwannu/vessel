//! Whole-pattern fallback over complete single-locale numeric preparation.
//! In particular, a missing compact message reselects the English scale and
//! digits as well as its suffix; no primary-language fragments leak into it.

use super::kernel_probe::{Amount, KernelError, NumberKernel};

#[derive(Debug, thiserror::Error)]
#[error("numeric pattern failed in the selected language ({selected}) and English ({english})")]
pub(super) struct FallbackError {
    selected: KernelError,
    english: KernelError,
}

#[derive(Debug)]
pub(super) struct FallbackNumbers<'a> {
    selected: NumberKernel<'a>,
    english: NumberKernel<'a>,
}

impl<'a> FallbackNumbers<'a> {
    pub(super) fn new(
        selected: NumberKernel<'a>,
        english: NumberKernel<'a>,
    ) -> Result<Self, KernelError> {
        english.require_complete()?;
        Ok(Self { selected, english })
    }

    pub(super) fn amount(&self, amount: Amount) -> Result<String, FallbackError> {
        match self.selected.amount(amount) {
            Ok(value) => Ok(value),
            Err(selected) => self
                .english
                .amount(amount)
                .map_err(|english| FallbackError { selected, english }),
        }
    }
}

#[cfg(test)]
#[path = "fallback_probe_tests.rs"]
mod tests;
