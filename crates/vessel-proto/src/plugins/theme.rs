//! Small semantic palettes; no CSS or executable theme content enters paint.

use std::{fmt, str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};

use super::{ContributionId, InvalidPluginValue, ThemeName};
use crate::key::serde_via_text;

const SHORT_HEX_DIGITS: usize = 3;
const RGB_HEX_DIGITS: usize = 6;
const RGBA_HEX_DIGITS: usize = 8;
const HEX_NIBBLE_BITS: u32 = 4;
const OPAQUE_ALPHA: u32 = 0xff;
const BYTE_BITS: u32 = 8;
const CHANNEL_DUPLICATE: u32 = 17;

/// A validated Paseo palette color, converted to RGBA only at the kit adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct HexColor {
    text: Arc<str>,
    rgba: u32,
}

impl HexColor {
    /// The original validated spelling, for catalog hashing and serialization.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Packed RRGGBBAA; no parsing failure remains after construction.
    #[must_use]
    pub const fn rgba(&self) -> u32 {
        self.rgba
    }
}

impl FromStr for HexColor {
    type Err = InvalidPluginValue;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let digits = raw.strip_prefix('#').ok_or(InvalidPluginValue::Color)?;
        if !matches!(
            digits.len(),
            SHORT_HEX_DIGITS | RGB_HEX_DIGITS | RGBA_HEX_DIGITS
        ) || !digits.bytes().all(|digit| digit.is_ascii_hexdigit())
        {
            return Err(InvalidPluginValue::Color);
        }
        let value = digits.chars().try_fold(0_u32, |value, digit| {
            let nibble = digit.to_digit(16).ok_or(InvalidPluginValue::Color)?;
            // At most RGBA_HEX_DIGITS nibbles; checked accumulation also
            // guards this invariant if the accepted widths ever change.
            value
                .checked_shl(HEX_NIBBLE_BITS)
                .and_then(|base| base.checked_add(nibble))
                .ok_or(InvalidPluginValue::Color)
        })?;
        let rgba = match digits.len() {
            SHORT_HEX_DIGITS => expand_short(value),
            RGB_HEX_DIGITS => value.wrapping_shl(BYTE_BITS) | OPAQUE_ALPHA,
            RGBA_HEX_DIGITS => value,
            _ => return Err(InvalidPluginValue::Color),
        };
        Ok(Self {
            text: Arc::from(raw),
            rgba,
        })
    }
}

impl fmt::Display for HexColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

fn expand_short(value: u32) -> u32 {
    let red = value.wrapping_shr(HEX_NIBBLE_BITS.saturating_mul(2)) & 0xf;
    let green = value.wrapping_shr(HEX_NIBBLE_BITS) & 0xf;
    let blue = value & 0xf;
    // Valid nibbles fit in each byte; saturation prevents arithmetic panic
    // if this helper is later reused with a wider channel representation.
    red.saturating_mul(CHANNEL_DUPLICATE)
        .wrapping_shl(BYTE_BITS.saturating_mul(3))
        | green
            .saturating_mul(CHANNEL_DUPLICATE)
            .wrapping_shl(BYTE_BITS.saturating_mul(2))
        | blue
            .saturating_mul(CHANNEL_DUPLICATE)
            .wrapping_shl(BYTE_BITS)
        | OPAQUE_ALPHA
}

serde_via_text!(HexColor);

/// Determines the core defaults retained for destructive and status colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    /// Light core semantic defaults.
    Light,
    /// Dark core semantic defaults.
    Dark,
}

/// The exact small palette Paseo permits; arbitrary paint tokens are refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ThemeColors {
    /// Main surface and accent foreground.
    pub background: HexColor,
    /// Main text; also the accent when no explicit accent was supplied.
    pub foreground: HexColor,
    /// Elevated surfaces.
    pub raised: HexColor,
    /// Interactive control surfaces.
    pub control: HexColor,
    /// Surface boundaries.
    pub border: HexColor,
    /// Optional accent, never an implicit brand substitution.
    pub accent: Option<HexColor>,
    /// Secondary text.
    pub muted_foreground: HexColor,
    /// Focus ring and extra-muted core text.
    pub ring: HexColor,
}

impl ThemeColors {
    /// Paseo defaults the accent to foreground without changing the catalog.
    #[must_use]
    pub fn effective_accent(&self) -> &HexColor {
        self.accent.as_ref().unwrap_or(&self.foreground)
    }
}

/// A data-only theme declaration; its host is supplied by the catalog owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ThemeContribution {
    /// Stable contribution identity, independent of translated name.
    pub id: ContributionId,
    /// The contribution's resolved display name.
    pub name: ThemeName,
    /// Which core semantic defaults are retained.
    pub appearance: Appearance,
    /// Only these semantic colors may enter the theme adapter.
    pub colors: ThemeColors,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_preserve_paseo_widths_alpha_and_short_expansion() {
        for (text, rgba) in [
            ("#aBc", 0xaabb_ccff),
            ("#123456", 0x1234_56ff),
            ("#12345678", 0x1234_5678),
        ] {
            assert_eq!(
                text.parse::<HexColor>().expect("fixture color").rgba(),
                rgba
            );
        }
        for text in [
            "#ab",
            "#abcd",
            "#1234567",
            "red",
            " #123",
            "#ggg",
            "#１２３",
        ] {
            assert_eq!(text.parse::<HexColor>(), Err(InvalidPluginValue::Color));
        }
    }

    #[test]
    fn unknown_theme_fields_and_executable_colors_are_rejected() {
        let valid = r##"{"id":"custom","name":" Custom ","appearance":"dark","colors":{"background":"#123","foreground":"#fff","raised":"#222","control":"#333","border":"#444","mutedForeground":"#aaa","ring":"#777"}}"##;
        let parsed: ThemeContribution = serde_json::from_str(valid).expect("fixture");
        assert_eq!(parsed.name.as_str(), "Custom");
        assert_eq!(parsed.colors.effective_accent(), &parsed.colors.foreground);
        let extra = valid.replacen(
            "\"appearance\"",
            "\"Component\":\"evil()\",\"appearance\"",
            1,
        );
        assert!(serde_json::from_str::<ThemeContribution>(&extra).is_err());
        let css = valid.replace("#123", "url(secret)");
        assert!(serde_json::from_str::<ThemeContribution>(&css).is_err());
    }
}
