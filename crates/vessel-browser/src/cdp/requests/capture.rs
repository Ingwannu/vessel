//! Full-page CSS geometry is checked before a raster-producing CDP request.
use super::{Captured, DeviceScale, Method, Request, sealed};
use serde::{Deserialize, Serialize};
use vessel_proto::browser::{InvalidBrowserBound, MAX_VIEWPORT_SIDE, ViewportSide, ViewportSize};

/// Pinned Chrome exposes CSS content dimensions; deprecated device units are not a fallback.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LayoutMetrics {
    css_content_size: ContentSize,
}
impl LayoutMetrics {
    pub(crate) const fn size(&self) -> ViewportSize {
        ViewportSize {
            width: self.css_content_size.width.0,
            height: self.css_content_size.height.0,
        }
    }
}
#[derive(Debug, Deserialize)]
struct ContentSize {
    width: ContentSide,
    height: ContentSide,
}
#[derive(Debug, Deserialize)]
#[serde(try_from = "f64")]
struct ContentSide(ViewportSide);
impl TryFrom<f64> for ContentSide {
    type Error = GeometryError;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if !value.is_finite() || value <= 0.0 || value > f64::from(MAX_VIEWPORT_SIDE) {
            return Err(GeometryError::Range);
        }
        // Two bounded metadata dimensions are not a hot pixel path. Use std's
        // checked parser after decimal rounding rather than an unsafe/numeric
        // cast or a new conversion dependency. No untrusted exponent survives.
        let rounded = value.ceil();
        let pixels = format!("{rounded:.0}")
            .parse::<u32>()
            .map_err(GeometryError::Parse)?;
        ViewportSide::try_from(pixels)
            .map(Self)
            .map_err(GeometryError::Side)
    }
}
#[derive(Debug, thiserror::Error)]
enum GeometryError {
    #[error("browser content dimension is outside the CSS surface bound")]
    Range,
    #[error("browser rounded CSS dimension could not be converted")]
    Parse(#[source] std::num::ParseIntError),
    #[error("browser CSS dimension exceeds the viewport side boundary")]
    Side(#[source] InvalidBrowserBound),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CapturePage {
    format: Format,
    capture_beyond_viewport: bool,
    clip: Clip,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Format {
    Png,
}
#[derive(Debug, Serialize)]
struct Clip {
    x: Origin,
    y: Origin,
    width: ViewportSide,
    height: ViewportSide,
    scale: DeviceScale,
}
#[derive(Debug, Serialize)]
#[serde(transparent)]
struct Origin(u8);
impl CapturePage {
    /// The Page owner must admit the combined RGBA area before constructing this request.
    pub(crate) const fn new(size: ViewportSize) -> Self {
        Self {
            format: Format::Png,
            capture_beyond_viewport: true,
            clip: Clip {
                x: Origin(0),
                y: Origin(0),
                width: size.width,
                height: size.height,
                scale: DeviceScale(1),
            },
        }
    }
}
impl sealed::Sealed for CapturePage {}
impl Request for CapturePage {
    type Response = Captured;
    const METHOD: Method = Method::Capture;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_css_extents_round_up_without_accepting_unbounded_or_device_units() {
        let metrics: LayoutMetrics = serde_json::from_str(
            r#"{"cssContentSize":{"x":0,"y":0,"width":390.25,"height":1700.01}}"#,
        )
        .unwrap();
        assert_eq!(metrics.size().width.get(), 391);
        assert_eq!(metrics.size().height.get(), 1701);
        for dimension in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -1.0,
            0.0,
            f64::from(MAX_VIEWPORT_SIDE) + 0.01,
        ] {
            assert!(ContentSide::try_from(dimension).is_err());
        }
        assert!(
            serde_json::from_str::<LayoutMetrics>(r#"{"contentSize":{"width":390,"height":844}}"#)
                .is_err()
        );
        let encoded = serde_json::to_value(CapturePage::new(metrics.size())).unwrap();
        assert_eq!(encoded["format"], "png");
        assert_eq!(encoded["captureBeyondViewport"], true);
        assert_eq!(
            encoded["clip"],
            serde_json::json!({"x":0,"y":0,"width":391,"height":1701,"scale":1})
        );
    }
}
