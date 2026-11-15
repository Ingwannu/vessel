//! Human page input is an explicit channel, never arbitrary CDP or tool history.

use serde::{Deserialize, Serialize};

use super::{CssDelta, InputText, MouseButton, RequiredInput, ViewportSize};
use crate::conversation::FiniteNumber;

/// A finite nonnegative CSS coordinate, bounded before queuing input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "FiniteNumber", into = "FiniteNumber")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
pub struct CssCoordinate(FiniteNumber);

/// Invalid input geometry never reflects the selected page's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("pointer coordinate is outside the supported CSS viewport")]
pub struct InvalidCssCoordinate;

impl TryFrom<FiniteNumber> for CssCoordinate {
    type Error = InvalidCssCoordinate;
    fn try_from(value: FiniteNumber) -> Result<Self, Self::Error> {
        if (0.0..f64::from(super::MAX_VIEWPORT_SIDE)).contains(&value.get()) {
            Ok(Self(value))
        } else {
            Err(InvalidCssCoordinate)
        }
    }
}
impl From<CssCoordinate> for FiniteNumber {
    fn from(value: CssCoordinate) -> Self {
        value.0
    }
}
impl CssCoordinate {
    /// Explicit geometry adapters may inspect the checked CSS quantity.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0.get()
    }
}

/// Pointer position is relative to the page viewport, excluding toolbar/letterboxing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CssPoint {
    /// Horizontal CSS coordinate.
    pub x: CssCoordinate,
    /// Vertical CSS coordinate.
    pub y: CssCoordinate,
}
impl CssPoint {
    /// Recheck against the target's current viewport immediately before dispatch.
    #[must_use]
    pub fn inside(self, viewport: ViewportSize) -> bool {
        self.x.get() < f64::from(viewport.width.get())
            && self.y.get() < f64::from(viewport.height.get())
    }
}

/// Press/release pairing belongs to the target actor, never a global desktop input API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "button", rename_all = "snake_case")]
pub enum PointerAction {
    /// Pointer movement without inventing a click.
    Move,
    /// A trusted press of this button.
    Press(MouseButton),
    /// A matching release, or cleanup when the pane loses capture.
    Release(MouseButton),
}

/// Key-up events cannot accidentally execute another complete key chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum KeyAction {
    /// Start/continue the key's pressed state.
    Down,
    /// Release the previously pressed key.
    Up,
}

/// Fixed human input vocabulary; text Debug is redacted by its shared newtype.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HumanInput {
    /// Coordinates are recomputed from the displayed frame, then checked again server-side.
    Pointer {
        /// Point in the current CSS viewport.
        position: CssPoint,
        /// Movement, press or release.
        action: PointerAction,
        /// Modifiers from the same human input event.
        modifiers: super::KeyModifiers,
    },
    /// Scrolling retains the pointer's page-relative position.
    Wheel {
        /// Point under the wheel event.
        position: CssPoint,
        /// Horizontal CSS-pixel delta.
        delta_x: CssDelta,
        /// Vertical CSS-pixel delta.
        delta_y: CssDelta,
    },
    /// Direct page typing travels only over the authenticated encrypted control link.
    Text {
        /// Never persisted into layout, diagnostics or a tool call's raw input.
        text: InputText,
    },
    /// Keyboard event names are decoded by the browser adapter, never by a shell.
    Key {
        /// Bounded by the transport envelope; never included in diagnostic output.
        key: RequiredInput,
        /// Press or release is explicit.
        action: KeyAction,
        /// Current modifiers.
        modifiers: super::KeyModifiers,
    },
    /// Lost focus closes any outstanding pointer/key press state in the target actor.
    ReleaseAll,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_are_finite_bounded_and_rechecked_after_resize() {
        for value in [-1.0, f64::from(super::super::MAX_VIEWPORT_SIDE)] {
            assert!(CssCoordinate::try_from(FiniteNumber::new(value).unwrap()).is_err());
        }
        assert!(serde_json::from_str::<CssCoordinate>("1e999").is_err());
        let point: CssPoint = serde_json::from_str(r#"{"x":100,"y":50}"#).unwrap();
        let viewport = |width, height| ViewportSize {
            width: super::super::ViewportSide::try_from(width).unwrap(),
            height: super::super::ViewportSide::try_from(height).unwrap(),
        };
        assert!(point.inside(viewport(101, 51)));
        assert!(!point.inside(viewport(100, 51)));
        assert!(!point.inside(viewport(101, 50)));
    }
}
