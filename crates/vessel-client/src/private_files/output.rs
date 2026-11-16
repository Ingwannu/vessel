//! Bounded streaming document formatting, still private test preparation.
//!
//! Existing private-file `Text`/`ByteLimit`/`Error` own decoding and safe errors.
//! No live file writer, lock or worker is wired by this module.
//! Unlike checking a finished `String`, each chunk is refused before append.

use std::fmt::{self, Write as _};

use super::{ByteLimit, Error, Text};

const MAX_BOOK_OUTPUT_BYTES: ByteLimit = ByteLimit(1_048_576);

struct Output {
    text: String,
    limit: ByteLimit,
    fault: Option<Error>,
}

/// Format a book directly into its one capped buffer, including comments.
/// This is formatting only; a successful value is not a file commit receipt.
pub(super) fn book(value: &impl fmt::Display) -> Result<Text, Error> {
    write_limited(value, MAX_BOOK_OUTPUT_BYTES)
}

/// A formatter's own failure and a sticky sink fault both refuse the document.
/// The caller supplies a streaming `Display`, never an already rendered `String`.
fn write_limited(value: &impl fmt::Display, limit: ByteLimit) -> Result<Text, Error> {
    let maximum = usize::try_from(limit.0).map_err(Error::Capacity)?;
    let mut text = String::new();
    // Rare file workers reserve one bounded output up front, as the input
    // reader does. Exact per-chunk growth would permit quadratic copying.
    text.try_reserve_exact(maximum).map_err(Error::Allocation)?;
    let mut output = Output {
        text,
        limit,
        fault: None,
    };
    let formatted = write!(&mut output, "{value}");
    let Output { text, fault, .. } = output;
    if let Some(error) = fault {
        return Err(error);
    }
    formatted.map_err(Error::Format)?;
    Ok(Text(text))
}

impl fmt::Write for Output {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.fault.is_some() {
            return Err(fmt::Error);
        }
        match self.append(text) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.fault = Some(error);
                Err(fmt::Error)
            }
        }
    }
}

impl Output {
    fn append(&mut self, text: &str) -> Result<(), Error> {
        let maximum = usize::try_from(self.limit.0).map_err(Error::Capacity)?;
        let length = self
            .text
            .len()
            .checked_add(text.len())
            .ok_or(Error::PlatformLimit)?;
        if length > maximum {
            return Err(Error::TooLarge { limit: self.limit });
        }
        self.text.push_str(text);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
