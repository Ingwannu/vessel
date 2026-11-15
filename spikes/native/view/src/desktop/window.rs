//! Reachability rules keep restored windows usable after a monitor is removed.

use super::zoom::ZoomLevel;
use serde::{Deserialize, Serialize};

/// Paseo's default width in device-independent pixels.
pub const DEFAULT_WIDTH: f64 = 1200.0;
/// Paseo's default height in device-independent pixels.
pub const DEFAULT_HEIGHT: f64 = 800.0;
/// The reference's minimum width; compact capture overrides remain available.
pub const MIN_WIDTH: f64 = 400.0;
/// The reference's minimum height.
pub const MIN_HEIGHT: f64 = 300.0;
const VISIBLE_WIDTH: f64 = 100.0;
const VISIBLE_HEIGHT: f64 = 80.0;
/// Coalescing interval used by the platform persistence owner.
pub const SAVE_DELAY: std::time::Duration = std::time::Duration::from_millis(400);

/// An OS-reported usable rectangle, validated before restoration uses it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkArea {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl WorkArea {
    /// Reject unusable display geometry rather than making a window unreachable.
    #[must_use]
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Option<Self> {
        (x.is_finite()
            && y.is_finite()
            && width.is_finite()
            && height.is_finite()
            && width > 0.0
            && height > 0.0)
            .then_some(Self {
                x,
                y,
                width,
                height,
            })
    }
}

/// Native maximization and fullscreen are distinct restored presentation modes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Presentation {
    /// Ordinary moveable window.
    #[default]
    Windowed,
    /// Preserve the ordinary restore bounds underneath maximization.
    Maximized,
    /// Preserve the ordinary restore bounds underneath fullscreen.
    Fullscreen,
}

/// Validated dimensions and an optional complete position, never half a position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct WindowState {
    position: Option<(f64, f64)>,
    width: f64,
    height: f64,
    presentation: Presentation,
    zoom: ZoomLevel,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredState {
    position: Option<(f64, f64)>,
    width: f64,
    height: f64,
    #[serde(default)]
    presentation: Presentation,
    #[serde(default)]
    zoom: ZoomLevel,
}

impl<'de> Deserialize<'de> for WindowState {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let raw = StoredState::deserialize(decoder)?;
        Self::new(raw.position, raw.width, raw.height, raw.presentation)
            .map(|state| state.with_zoom(raw.zoom))
            .ok_or_else(|| serde::de::Error::custom("invalid window geometry"))
    }
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            position: None,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            presentation: Presentation::Windowed,
            zoom: ZoomLevel::default(),
        }
    }
}

impl WindowState {
    /// Parse finite geometry at the platform/file boundary; clamp small dimensions.
    #[must_use]
    pub fn new(
        position: Option<(f64, f64)>,
        width: f64,
        height: f64,
        presentation: Presentation,
    ) -> Option<Self> {
        if !width.is_finite()
            || !height.is_finite()
            || position.is_some_and(|(x, y)| !x.is_finite() || !y.is_finite())
        {
            return None;
        }
        Some(Self {
            position,
            width: width.max(MIN_WIDTH),
            height: height.max(MIN_HEIGHT),
            presentation,
            zoom: ZoomLevel::default(),
        })
    }

    /// Position exists only when the saved rectangle is still reachable.
    #[must_use]
    pub const fn position(self) -> Option<(f64, f64)> {
        self.position
    }
    /// Validated device-independent dimensions for a platform window.
    #[must_use]
    pub const fn dimensions(self) -> (f64, f64) {
        (self.width, self.height)
    }
    /// Presentation is restored separately from ordinary bounds.
    #[must_use]
    pub const fn presentation(self) -> Presentation {
        self.presentation
    }

    /// Authored half-steps are independent of the platform display's DPI.
    #[must_use]
    pub const fn zoom(self) -> ZoomLevel {
        self.zoom
    }

    /// Keep the same validated geometry with an admitted per-window zoom.
    #[must_use]
    pub const fn with_zoom(self, zoom: ZoomLevel) -> Self {
        Self { zoom, ..self }
    }

    /// Keep the largest sufficiently visible overlap; otherwise ask the OS to place.
    #[must_use]
    #[expect(
        clippy::float_arithmetic,
        reason = "finite OS geometry requires DIP rectangle arithmetic"
    )]
    pub fn clamp(self, displays: &[WorkArea]) -> Self {
        let Some(primary) = displays.first() else {
            return Self {
                position: None,
                ..self
            };
        };
        let mut target = primary;
        let mut best_overlap = 0.0;
        if let Some((x, y)) = self.position {
            for display in displays {
                let width =
                    ((x + self.width).min(display.x + display.width) - x.max(display.x)).max(0.0);
                let height =
                    ((y + self.height).min(display.y + display.height) - y.max(display.y)).max(0.0);
                let overlap = width * height;
                if width >= VISIBLE_WIDTH.min(self.width)
                    && height >= VISIBLE_HEIGHT.min(self.height)
                    && overlap > best_overlap
                {
                    best_overlap = overlap;
                    target = display;
                }
            }
        }
        let width = self.width.min(target.width);
        let height = self.height.min(target.height);
        let position = self.position.filter(|_| best_overlap > 0.0).map(|(x, y)| {
            (
                x.clamp(target.x, target.x + target.width - width),
                y.clamp(target.y, target.y + target.height - height),
            )
        });
        Self {
            position,
            width,
            height,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_display_discards_position_and_small_display_limits_size() {
        let state = WindowState::new(
            Some((-2000.0, 20.0)),
            1600.0,
            900.0,
            Presentation::Maximized,
        )
        .unwrap();
        let clamped = state.clamp(&[WorkArea::new(0.0, 0.0, 800.0, 600.0).unwrap()]);
        assert_eq!(clamped.position(), None);
        assert_eq!(clamped.dimensions(), (800.0, 600.0));
        assert_eq!(clamped.presentation(), Presentation::Maximized);
    }

    #[test]
    fn reachable_secondary_is_kept_and_clamped_inside_its_area() {
        let state =
            WindowState::new(Some((-50.0, 10.0)), 900.0, 600.0, Presentation::Windowed).unwrap();
        let clamped = state.clamp(&[WorkArea::new(0.0, 0.0, 800.0, 600.0).unwrap()]);
        assert_eq!(clamped.position(), Some((0.0, 0.0)));
        assert_eq!(clamped.dimensions(), (800.0, 600.0));
        assert_eq!(state.clamp(&[]).position(), None);
    }

    #[test]
    fn invalid_geometry_never_crosses_the_boundary() {
        assert!(WindowState::new(None, f64::NAN, 1.0, Presentation::Windowed).is_none());
        assert!(WorkArea::new(0.0, 0.0, f64::INFINITY, 20.0).is_none());
        assert!(serde_json::from_str::<WindowState>(r#"{"width":1e999,"height":100}"#).is_err());
        assert_eq!(
            WindowState::new(None, 1.0, 1.0, Presentation::Windowed)
                .unwrap()
                .dimensions(),
            (MIN_WIDTH, MIN_HEIGHT)
        );
    }
}
