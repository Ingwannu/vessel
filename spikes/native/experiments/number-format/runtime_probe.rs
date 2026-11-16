//! Source input for the scratch View overlay, never loaded by product code.
//! Build the two numeric namespaces before any generated getter can bind them.

use std::{num::NonZeroUsize, sync::Arc};

use crate::{carrier_probe::NumberContext, kernel_probe::NumberKernel, placement_probe::Symbols};

use super::{FormatError, InitError, number};

pub(super) fn context(tag: &str, sources: &[&str]) -> Result<Arc<NumberContext>, InitError> {
    let language = tag
        .parse()
        .map_err(|error| InitError::Locale(Arc::new(error)))?;
    let english_sources = super::EMBEDDED
        .get(super::ENGLISH.index)
        .ok_or(InitError::MissingLocale)?
        .sources;
    let selected = prepare(tag, sources)?;
    let english = prepare("en", english_sources)?;
    NumberContext::new(language, selected, english)
        .map_err(|error| InitError::Plain(FormatError::Numeric(error.into())))
}

fn prepare(tag: &str, sources: &[&str]) -> Result<NumberKernel<'static>, InitError> {
    let rules = number::rules(tag).ok_or(InitError::FormatRules)?;
    let primary = NonZeroUsize::new(rules.primary).ok_or(InitError::FormatRules)?;
    let secondary = NonZeroUsize::new(rules.secondary).ok_or(InitError::FormatRules)?;
    let symbols = Symbols::new(rules.group, rules.decimal, rules.digits, primary, secondary)
        .map_err(|error| {
            InitError::Plain(FormatError::Numeric(
                crate::kernel_probe::KernelError::from(error).into(),
            ))
        })?;
    NumberKernel::new(tag, &sources.join("\n"), symbols)
        .map_err(|error| InitError::Plain(FormatError::Numeric(error.into())))
}
