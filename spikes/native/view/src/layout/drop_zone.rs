//! Pointer hit testing is independent of drag sensors and the renderer toolkit.
use super::{LayoutError, Position};
/// Paseo center zone occupies the middle forty percent of both axes.
pub const DROP_CENTER_RATIO: f32 = 0.4;
/// An outer edge is selected before the nearest-edge fallback.
pub const DROP_EDGE_RATIO: f32 = 0.15;
const CENTER_START: f32 = (1.0 - DROP_CENTER_RATIO) / 2.0;
const CENTER_END: f32 = 1.0 - CENTER_START;
/// Validated pane-local pointer fractions; nonfinite or zero-size bounds refuse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropPoint {
    x: f32,
    y: f32,
}
impl DropPoint {
    /// Convert the toolkit's measured points once at the UI edge.
    ///
    /// # Errors
    /// Refuses nonfinite geometry, empty bounds or unrepresentable normalization.
    pub fn in_pane(x: f32, y: f32, width: f32, height: f32) -> Result<Self, LayoutError> {
        if ![x, y, width, height].iter().all(|value| value.is_finite())
            || width <= 0.0
            || height <= 0.0
        {
            return Err(LayoutError::InvalidState);
        }
        let (inverse_width, inverse_height) = (width.recip(), height.recip());
        if !inverse_width.is_finite() || !inverse_height.is_finite() {
            return Err(LayoutError::InvalidState);
        }
        Ok(Self {
            x: x.mul_add(inverse_width, 0.0).clamp(0.0, 1.0),
            y: y.mul_add(inverse_height, 0.0).clamp(0.0, 1.0),
        })
    }
    /// Reference center/edge rules keep drop preview and application identical.
    #[must_use]
    pub fn position(self) -> Position {
        if (CENTER_START..=CENTER_END).contains(&self.x)
            && (CENTER_START..=CENTER_END).contains(&self.y)
        {
            return Position::Center;
        }
        let right = self.x.mul_add(-1.0, 1.0);
        let bottom = self.y.mul_add(-1.0, 1.0);
        if self.x <= DROP_EDGE_RATIO {
            return Position::Left;
        }
        if right <= DROP_EDGE_RATIO {
            return Position::Right;
        }
        if self.y <= DROP_EDGE_RATIO {
            return Position::Top;
        }
        if bottom <= DROP_EDGE_RATIO {
            return Position::Bottom;
        }
        if self.x <= right && self.x <= self.y && self.x <= bottom {
            Position::Left
        } else if right <= self.y && right <= bottom {
            Position::Right
        } else if self.y <= bottom {
            Position::Top
        } else {
            Position::Bottom
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn center_and_four_edges_match_preview_policy() {
        for (x, y, expected) in [
            (50.0, 50.0, Position::Center),
            (5.0, 50.0, Position::Left),
            (95.0, 50.0, Position::Right),
            (50.0, 5.0, Position::Top),
            (50.0, 95.0, Position::Bottom),
        ] {
            assert_eq!(
                DropPoint::in_pane(x, y, 100.0, 100.0).unwrap().position(),
                expected
            );
        }
        assert!(DropPoint::in_pane(f32::NAN, 0.0, 100.0, 100.0).is_err());
        assert!(DropPoint::in_pane(0.0, 0.0, 0.0, 100.0).is_err());
    }
}
