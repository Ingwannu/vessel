//! Page content has explicit memory bounds and no diagnostic rendering.

use std::fmt;
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Paseo's rendered snapshot/evaluation bound; measured in UTF-8 bytes here.
pub const MAX_PAGE_TEXT_BYTES: usize = 80_000;
/// Encoded transport must not hide an image larger than this decoded PNG payload.
pub const MAX_SCREENSHOT_BYTES: usize = 8 * 1024 * 1024;
/// Reject an oversized string before allocating the base64 decoder's output.
const MAX_ENCODED_SCREENSHOT_BYTES: usize = MAX_SCREENSHOT_BYTES.div_ceil(3).saturating_mul(4);
/// A PNG response must identify its format, not merely contain arbitrary base64.
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Refusals retain their library cause without reflecting the page or payload.
#[derive(Debug, thiserror::Error)]
pub enum InvalidBrowserContent {
    /// Output must fit before it can be retained or sent.
    #[error("browser text exceeds {MAX_PAGE_TEXT_BYTES} UTF-8 bytes")]
    TextSize,
    /// The bound applies before and after base64 decoding.
    #[error("screenshot exceeds {MAX_SCREENSHOT_BYTES} decoded PNG bytes")]
    ImageSize,
    /// An empty response or other image format is not a successful screenshot.
    #[error("screenshot is not a PNG payload")]
    ImageFormat,
    /// Keep the decoder cause rather than converting it to an untyped string.
    #[error("screenshot encoding is invalid")]
    Encoding(#[source] base64::DecodeError),
}

/// Bounded page text can contain signed-in data; Debug is intentionally redacted.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct PageText(Arc<str>);

impl PageText {
    /// Only the target's content renderer should expose these bytes.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Truncation is explicit and always cuts on a UTF-8 character boundary.
    #[must_use]
    pub fn capped(text: &str) -> CappedText {
        if text.len() <= MAX_PAGE_TEXT_BYTES {
            return CappedText {
                text: Self(Arc::from(text)),
                completeness: TextCompleteness::Whole,
            };
        }
        let mut prefix = String::with_capacity(MAX_PAGE_TEXT_BYTES);
        for character in text.chars() {
            // Saturation treats an impossible count overflow as over-budget.
            if prefix.len().saturating_add(character.len_utf8()) > MAX_PAGE_TEXT_BYTES {
                break;
            }
            prefix.push(character);
        }
        CappedText {
            text: Self(Arc::from(prefix)),
            completeness: TextCompleteness::Truncated,
        }
    }
}

impl TryFrom<String> for PageText {
    type Error = InvalidBrowserContent;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > MAX_PAGE_TEXT_BYTES {
            Err(InvalidBrowserContent::TextSize)
        } else {
            Ok(Self(Arc::from(value)))
        }
    }
}
impl From<PageText> for String {
    fn from(value: PageText) -> Self {
        value.0.to_string()
    }
}
impl fmt::Debug for PageText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PageText([redacted])")
    }
}

/// A limit must never make a partial page look like complete evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TextCompleteness {
    /// The complete result fit its bound.
    Whole,
    /// A visible warning/marker accompanies the bounded preview.
    Truncated,
}

/// Text and completeness travel together so callers cannot lose the truncation fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CappedText {
    /// At most the named byte bound, without cutting a character.
    pub text: PageText,
    /// A partial result is never rendered as a complete page snapshot.
    pub completeness: TextCompleteness,
}

/// Image bytes are opaque PNG content, not an untyped structured-data channel.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ScreenshotPng(Bytes);

impl ScreenshotPng {
    /// Validate the transport bound and format before retaining a CDP result.
    ///
    /// # Errors
    /// Refuses oversized or non-PNG payloads; decoding pixels remains the UI boundary's job.
    pub fn from_png(bytes: Bytes) -> Result<Self, InvalidBrowserContent> {
        if bytes.len() > MAX_SCREENSHOT_BYTES {
            return Err(InvalidBrowserContent::ImageSize);
        }
        if !bytes.starts_with(PNG_SIGNATURE) {
            return Err(InvalidBrowserContent::ImageFormat);
        }
        Ok(Self(bytes))
    }

    /// Image content reaches an explicit image renderer or MCP image channel only.
    #[must_use]
    pub fn bytes(&self) -> &Bytes {
        &self.0
    }
}

impl TryFrom<String> for ScreenshotPng {
    type Error = InvalidBrowserContent;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > MAX_ENCODED_SCREENSHOT_BYTES {
            return Err(InvalidBrowserContent::ImageSize);
        }
        let bytes = STANDARD
            .decode(value)
            .map_err(InvalidBrowserContent::Encoding)?;
        Self::from_png(Bytes::from(bytes))
    }
}
impl From<ScreenshotPng> for String {
    fn from(value: ScreenshotPng) -> Self {
        STANDARD.encode(value.bytes())
    }
}
impl fmt::Debug for ScreenshotPng {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ScreenshotPng([redacted])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_truncation_is_bounded_and_cannot_disappear_in_diagnostics() {
        let text = "한".repeat(MAX_PAGE_TEXT_BYTES);
        let capped = PageText::capped(&text);
        assert_eq!(capped.completeness, TextCompleteness::Truncated);
        assert!(capped.text.as_str().len() <= MAX_PAGE_TEXT_BYTES);
        assert!(text.starts_with(capped.text.as_str()));
        assert!(!format!("{capped:?}").contains('한'));
        assert!(PageText::try_from(text).is_err());
    }

    #[test]
    fn image_bound_applies_before_retention_and_is_not_a_paint_claim() {
        let mut oversized = vec![0; MAX_SCREENSHOT_BYTES.saturating_add(1)];
        oversized
            .get_mut(..PNG_SIGNATURE.len())
            .unwrap()
            .copy_from_slice(PNG_SIGNATURE);
        assert!(matches!(
            ScreenshotPng::from_png(Bytes::from(oversized)),
            Err(InvalidBrowserContent::ImageSize)
        ));
        assert!(ScreenshotPng::try_from("not-base64".to_owned()).is_err());
        assert!(ScreenshotPng::from_png(Bytes::from_static(b"not-png")).is_err());
    }
}
