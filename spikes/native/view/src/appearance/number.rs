//! Numeric input at the form edge: finite, floor, clamp. Stored sizes use
//! their existing validated preference types, not raw numeric strings.
use super::ContentPixels;
use crate::prefs::{CodeSize, ContentSize, InterfaceSize};

/// A finite number typed into a field, before commit-time bounds apply.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Number(f64);

// NaN cannot enter this type; finite numeric equality is reflexive.
impl Eq for Number {}

impl Number {
    /// Parse decimal/exponent or JavaScript's unsigned radix forms. Empty
    /// and nonfinite input restore the saved value instead of becoming zero.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let text = super::input::trim(text);
        if text.is_empty() {
            return None;
        }
        let value = match text.get(..2) {
            Some("0x" | "0X") => radix(text.get(2..)?, 16),
            Some("0b" | "0B") => radix(text.get(2..)?, 2),
            Some("0o" | "0O") => radix(text.get(2..)?, 8),
            Some(_) | None => text.parse::<f64>().ok(),
        }?;
        value.is_finite().then_some(Self(value))
    }

    /// Commit to the interface axis without compounding earlier scales.
    #[must_use]
    pub fn interface(self) -> Option<InterfaceSize> {
        let value = self.clamped(InterfaceSize::MIN.into(), InterfaceSize::MAX.into())?;
        InterfaceSize::try_from(u8::try_from(value).ok()?).ok()
    }

    /// Commit to the independent content axis.
    #[must_use]
    pub fn content(self) -> Option<ContentSize> {
        let value = self.clamped(ContentSize::MIN.into(), ContentSize::MAX.into())?;
        ContentSize::try_from(u8::try_from(value).ok()?).ok()
    }

    /// Commit to the independent code axis.
    #[must_use]
    pub fn code(self) -> Option<CodeSize> {
        let value = self.clamped(CodeSize::MIN.into(), CodeSize::MAX.into())?;
        CodeSize::try_from(u8::try_from(value).ok()?).ok()
    }

    /// Commit to the explicit content-width range.
    #[must_use]
    pub fn width(self) -> Option<ContentPixels> {
        ContentPixels::try_from(self.clamped(ContentPixels::MIN, ContentPixels::MAX)?).ok()
    }

    fn clamped(self, minimum: u16, maximum: u16) -> Option<u16> {
        // The UI also needs this normalized integer text. Parsing it avoids
        // an unchecked float-to-integer cast; bounds are tiny and finite.
        self.0
            .floor()
            .clamp(f64::from(minimum), f64::from(maximum))
            .to_string()
            .parse()
            .ok()
    }
}

fn radix(text: &str, base: u32) -> Option<f64> {
    if text.is_empty() {
        return None;
    }
    text.chars().try_fold(0.0_f64, |value, ch| {
        let digit = f64::from(ch.to_digit(base)?);
        let next = value.mul_add(f64::from(base), digit);
        next.is_finite().then_some(next)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_commit_matches_finite_floor_and_each_source_axis() {
        for text in [
            "",
            "  ",
            "NaN",
            "Infinity",
            "-Infinity",
            "1e9999",
            "1_000",
            "0x",
        ] {
            assert_eq!(Number::parse(text), None, "{text}");
        }
        assert_eq!(
            Number::parse("14.9").unwrap().interface().unwrap().get(),
            14
        );
        assert_eq!(Number::parse("0x10").unwrap().content().unwrap().get(), 16);
        assert_eq!(
            Number::parse("0b1010").unwrap().interface().unwrap().get(),
            InterfaceSize::MIN
        );
        assert_eq!(Number::parse("0o14").unwrap().code().unwrap().get(), 12);
        assert_eq!(
            Number::parse("-99").unwrap().code().unwrap().get(),
            CodeSize::MIN
        );
        assert_eq!(
            Number::parse("10000").unwrap().content().unwrap().get(),
            ContentSize::MAX
        );
        assert_eq!(
            Number::parse("21").unwrap().interface().unwrap().get(),
            InterfaceSize::MAX
        );
        assert_eq!(
            Number::parse("22").unwrap().code().unwrap().get(),
            CodeSize::MAX
        );
    }
}
