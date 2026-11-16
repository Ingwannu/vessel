//! Typed source placement/timing, preserving every existing composer's default.
use gpui::{Anchor, Bounds, Pixels, Point, point, px};
use std::time::Duration;

/// A tooltip aligns to the horizontal center of its actual trigger bounds.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Existing composer tooltip position.
    #[default]
    Top,
    /// Pane toolbar default in the pinned source.
    Bottom,
}
/// Finite nonnegative distance from the trigger edge in logical points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Offset(Pixels);
impl Offset {
    /// Both pinned composer and pane toolbar profiles use eight logical points.
    pub const EIGHT: Self = Self(px(8.0));
}
/// Invalid geometry is refused at the caller's configuration edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidOffset {
    /// A tooltip cannot be placed at NaN or infinity.
    #[error("tooltip offset is not finite")]
    NotFinite,
    /// The side selects direction; the distance itself cannot be negative.
    #[error("tooltip offset is negative")]
    Negative,
}
impl TryFrom<Pixels> for Offset {
    type Error = InvalidOffset;
    fn try_from(value: Pixels) -> Result<Self, Self::Error> {
        let points = f32::from(value);
        if !points.is_finite() {
            return Err(InvalidOffset::NotFinite);
        }
        if points < 0.0 {
            return Err(InvalidOffset::Negative);
        }
        Ok(Self(value))
    }
}
/// Hover delay owned by the same screen tooltip controller, not an area timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delay(Duration);
impl Delay {
    /// Existing callers show immediately.
    pub const IMMEDIATE: Self = Self(Duration::ZERO);
    /// Pinned pane toolbar desktop hover delay.
    pub const TOOLBAR: Self = Self(Duration::from_millis(300));
    /// A typed duration, with no string or bare numeric unit crossing this API.
    #[must_use]
    pub const fn new(duration: Duration) -> Self {
        Self(duration)
    }
    pub(super) const fn duration(self) -> Duration {
        self.0
    }
}
/// Placement and timing are source facts; no separate overlay/controller exists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Configuration {
    side: Side,
    offset: Offset,
    delay: Delay,
}
impl Default for Configuration {
    fn default() -> Self {
        Self::new(Side::Top, Offset::EIGHT, Delay::IMMEDIATE)
    }
}
impl Configuration {
    /// Explicit source profile with validated distance and a typed delay.
    #[must_use]
    pub const fn new(side: Side, offset: Offset, delay: Delay) -> Self {
        Self {
            side,
            offset,
            delay,
        }
    }
    /// `pane-content-toolbar.tsx` defaults: bottom, offset eight, 300 milliseconds.
    #[must_use]
    pub const fn toolbar() -> Self {
        Self::new(Side::Bottom, Offset::EIGHT, Delay::TOOLBAR)
    }
    /// The source card-refresh control explicitly chooses top over the default.
    #[must_use]
    pub const fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }
    pub(super) const fn delay(self) -> Duration {
        self.delay.duration()
    }
    pub(super) fn placement(
        self,
        bounds: Bounds<Pixels>,
    ) -> (Point<Pixels>, Anchor, Point<Pixels>) {
        match self.side {
            Side::Top => (
                point(bounds.center().x, bounds.top()),
                Anchor::BottomCenter,
                // The validated finite distance selects a direction without
                // untyped float arithmetic or another screen geometry override.
                point(px(0.0), px(f32::from(self.offset.0).copysign(-1.0))),
            ),
            Side::Bottom => (
                point(bounds.center().x, bounds.bottom()),
                Anchor::TopCenter,
                point(px(0.0), self.offset.0),
            ),
        }
    }

    pub(super) fn placement_start(
        self,
        bounds: Bounds<Pixels>,
    ) -> (Point<Pixels>, Anchor, Point<Pixels>) {
        // Reuse the one vertical side/offset decision; Settings changes only
        // horizontal alignment against the same retained visual rectangle.
        let (mut position, _, offset) = self.placement(bounds);
        position.x = bounds.left();
        let anchor = match self.side {
            Side::Top => Anchor::BottomLeft,
            Side::Bottom => Anchor::TopLeft,
        };
        (position, anchor, offset)
    }
}
