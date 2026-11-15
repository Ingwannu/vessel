//! A measured single-open height shares authored stops' spring and drag math.
//! Equal authored percentages remain invalid: fitting is a separate typed mode.
use super::{Percentage, SnapPoints, Stop};
use gpui::Pixels;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Positions {
    Authored(SnapPoints),
    ContentFit(Measured),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Measured {
    fraction: f32,
}
impl Default for Positions {
    fn default() -> Self {
        Self::DEFAULT
    }
}
impl From<SnapPoints> for Positions {
    fn from(points: SnapPoints) -> Self {
        Self::Authored(points)
    }
}
impl Positions {
    pub(crate) const DEFAULT: Self = Self::Authored(SnapPoints::DEFAULT);
    pub(super) fn fit(content: Pixels, viewport: Pixels, cap: Percentage) -> Option<Self> {
        let content = content.as_f32();
        let viewport = viewport.as_f32();
        if !content.is_finite() || !viewport.is_finite() || content <= 0.0 || viewport <= 0.0 {
            return None;
        }
        let fraction = content.mul_add(viewport.recip(), 0.0).min(cap.fraction());
        // A positive measured pixel value may underflow in a pathological
        // viewport. Refuse it rather than dividing by zero in backdrop math.
        (fraction > 0.0).then_some(Self::ContentFit(Measured { fraction }))
    }
    pub(crate) fn position(self, stop: Stop) -> f32 {
        match self {
            Self::Authored(points) if points == SnapPoints::DEFAULT => stop.position(),
            Self::Authored(points) => points.position(stop),
            Self::ContentFit(measured) => match stop {
                Stop::Closed => 1.0,
                Stop::Compact | Stop::Expanded => measured.fraction.mul_add(-1.0, 1.0),
            },
        }
    }
    pub(super) fn compact_fraction(self) -> f32 {
        match self {
            Self::Authored(points) => points.compact().fraction(),
            Self::ContentFit(measured) => measured.fraction,
        }
    }
}
