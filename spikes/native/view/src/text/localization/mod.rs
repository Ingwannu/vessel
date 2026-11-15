//! Embedded translations (ADR 0039), alongside the legacy area tables.
//!
//! [Decision Log]
//! - Purpose: translators own sentences and grammar; callers own typed values.
//! - Constraint: view performs no I/O and a plain label must cost no more at
//!   render time than the old table field. Bundles are initialized explicitly
//!   before drawing; only generated field borrows are used for plain labels.
//! - Alternatives: runtime string keys lose compiler checks; compiling every
//!   sentence into code loses Fluent's asymmetric translation grammar.
//! - Choice: an English-generated API, concurrent Fluent bundles, and a
//!   retained plain-value struct per language. The process cache is bounded
//!   by the number of folders embedded at build time, plus one pseudo locale.
//! - Costs: variable messages allocate when formatted; call them when the
//!   underlying value changes, not while painting an unchanged frame.
//!
//! A key that does not exist does not compile:
//! ```compile_fail
//! use vessel_app_view::localization::{Translator, ENGLISH, InitError};
//! fn example() -> Result<(), InitError> {
//!     let t = Translator::get(ENGLISH)?;
//!     t.examples().unknown_key();
//!     Ok(())
//! }
//! ```
//! A missing argument does not compile:
//! ```compile_fail
//! use vessel_app_view::localization::{Translator, ENGLISH, InitError};
//! fn example() -> Result<(), InitError> {
//!     let t = Translator::get(ENGLISH)?;
//!     t.examples().welcome();
//!     Ok(())
//! }
//! ```

mod formats;
mod grammar;
mod language;
mod number;
mod runtime;

use fluent_bundle::FluentArgs;

pub use grammar::Particle;
pub use language::{LanguagePreference, LanguageTag, Locale, resolve_language, resolve_languages};
use runtime::{Bundle, Embedded, resolve};
pub use runtime::{FormatError, InitError, Translator};

/// A checked numeric message argument. Counts are exact `u32`; decimal values
/// use Fluent's floating-point representation and are not financial arithmetic.
#[derive(Debug, Clone, PartialEq)]
pub struct Number(fluent_bundle::types::FluentNumber);

/// A decimal argument exceeded the finite range this formatter supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("message decimals must be finite, within u32 magnitude, and at most nine decimal places")]
pub struct NumberError;

impl Number {
    /// All u32 counts are exactly representable by Fluent.
    #[must_use]
    pub fn count(value: u32) -> Self {
        Self(value.into())
    }

    /// Supply a decimal without making NaN/infinity or unbounded plural operands
    /// representable in the message API.
    /// # Errors
    /// Rejects non-finite values and magnitudes above `u32::MAX`.
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
        Ok(Self(value.into()))
    }

    fn fluent(self) -> fluent_bundle::FluentValue<'static> {
        self.0.into()
    }
}

/// Valid Gregorian date/local-clock parts. The engine serializes them only
/// at Fluent's value boundary; callers never supply an unchecked date string.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Date {
    /// A Gregorian calendar date, with no time or zone.
    Day(chrono::NaiveDate),
    /// A local clock reading; the caller supplies the already resolved zone label.
    Time {
        /// A valid local clock reading, with minute precision in patterns.
        time: chrono::NaiveTime,
        /// The caller-resolved zone name; no zone arithmetic occurs here.
        zone: String,
    },
    /// Local date and clock reading; this layer does not convert time zones.
    DateTime {
        /// A valid Gregorian date in the caller's chosen zone.
        date: chrono::NaiveDate,
        /// A valid local clock reading, with minute precision in patterns.
        time: chrono::NaiveTime,
        /// The caller-resolved zone name; no zone arithmetic occurs here.
        zone: String,
    },
}

impl Date {
    fn fluent(&self) -> Result<String, FormatError> {
        serde_json::to_string(self)
            .map_err(|error| FormatError::DateEncoding(std::sync::Arc::new(error)))
    }
}

// Generated from the parsed English resources. This is the only source
// of message identifiers in the public API; callers cannot supply strings.
include!(concat!(env!("OUT_DIR"), "/translations.rs"));

#[cfg(test)]
mod tests;

#[cfg(test)]
mod format_tests;

#[cfg(test)]
mod fixture;
