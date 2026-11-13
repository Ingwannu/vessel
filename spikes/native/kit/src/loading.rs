//! Paseo `LoadingSpinner`'s `ActivityIndicator` profile, separate from legacy arcs.
//!
//! [Decision Log]
//! - Purpose: one source-faithful activity indicator for schedules and settings.
//! - Constraint: existing Spinner vectors/callers keep their geometry and800ms.
//! - Alternatives: an area-owned loader, or silently repaint every old spinner.
//! - Choice: additive size/profile using the same SVG and animation renderer;
//!   base ring20%, foreground arc20 units on radius14,750ms linear rotation.
//! - Why: `LoadingSpinner` delegates to `RNWeb` `ActivityIndicator`, whose large36,
//!   small20 and numeric sizes differ from the existing24-grid arc.
//! - Cost: callers explicitly migrate; numeric size is validated once at edge.
use crate::assets::Glyph;
use gpui::prelude::*;
use gpui::{Animation, AnimationExt as _, AnyElement, ElementId, Hsla, Pixels, Svg, px, svg};
use std::time::Duration;

/// A finite, positive numeric source size; fractional points remain exact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Points(Pixels);
impl Points {
    /// Exact validated dimension, for geometry shared with a containing slot.
    #[must_use]
    pub const fn pixels(self) -> Pixels {
        self.0
    }

    /// `ProviderRow`/`StatusIndicator`'s explicit numeric10 refresh indicator.
    pub const TEN: Self = Self(px(10.0));
    /// `MenuItem`'s explicit numeric16 pending indicator.
    pub const SIXTEEN: Self = Self(px(16.0));
}
/// A programmer-supplied numeric size cannot become invalid SVG geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidPoints {
    /// NaN or infinity is not a layout dimension.
    #[error("activity indicator size is not finite")]
    NotFinite,
    /// Zero/negative dimensions cannot display an indicator.
    #[error("activity indicator size must be positive")]
    NotPositive,
}
impl TryFrom<Pixels> for Points {
    type Error = InvalidPoints;
    fn try_from(value: Pixels) -> Result<Self, Self::Error> {
        let number = f32::from(value);
        if !number.is_finite() {
            return Err(InvalidPoints::NotFinite);
        }
        if number <= 0.0 {
            return Err(InvalidPoints::NotPositive);
        }
        Ok(Self(value))
    }
}
/// Source named sizes, plus a checked numeric dimension.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    /// `ActivityIndicator` small20.
    Small,
    /// `ActivityIndicator` large36.
    Large,
    /// Exact source numeric size, including fractional points.
    Numeric(Points),
}
impl Size {
    const fn side(self) -> Pixels {
        match self {
            Self::Small => px(20.0),
            Self::Large => px(36.0),
            Self::Numeric(points) => points.0,
        }
    }
}
const PERIOD: Duration = Duration::from_millis(750);
pub(crate) const LEGACY_PERIOD: Duration = Duration::from_millis(800);
fn vector(size: Size, colour: Hsla) -> Svg {
    svg()
        .path(Glyph::ActivityIndicator.path())
        .size(size.side())
        .flex_shrink_0()
        .text_color(colour)
}
/// The reference wrapper's always-animating indicator. Its current consumers
/// expose colour/size, not `ActivityIndicator`'s unused stop/hide flags.
#[must_use]
pub fn activity_indicator(id: impl Into<ElementId>, size: Size, colour: Hsla) -> AnyElement {
    rotate(id, vector(size, colour), PERIOD)
}
fn motion(period: Duration) -> Animation {
    Animation::new(period).repeat()
}
/// A single motion path; the legacy spinner explicitly keeps its old period.
pub(crate) fn rotate(id: impl Into<ElementId>, vector: Svg, period: Duration) -> AnyElement {
    vector
        .with_animation(id, motion(period), |vector, delta| {
            vector.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta)))
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Scheme, Tokens};
    #[test]
    fn sizes_and_actual_vector_ink_match_the_source() {
        assert_eq!(Size::Small.side(), px(20.0));
        assert_eq!(Size::Large.side(), px(36.0));
        assert_eq!(Size::Numeric(Points::TEN).side(), px(10.0));
        assert_eq!(Size::Numeric(Points::SIXTEEN).side(), px(16.0));
        assert_eq!(
            Size::Numeric(Points::try_from(px(16.5)).unwrap()).side(),
            px(16.5)
        );
        assert_eq!(PERIOD, Duration::from_millis(750));
        for scheme in [Scheme::Light, Scheme::Dark] {
            let ink = Tokens::of(scheme).neutral.foreground_muted;
            let mut actual = vector(Size::Large, ink);
            assert_eq!(actual.style().size.width, Some(px(36.0).into()));
            assert_eq!(actual.style().size.height, Some(px(36.0).into()));
            assert_eq!(actual.style().text.color, Some(ink));
        }
    }
    #[test]
    fn motion_keeps_both_periods_linear_and_respects_the_shared_app_pipeline() {
        assert_eq!(PERIOD, Duration::from_millis(750));
        assert_eq!(LEGACY_PERIOD, Duration::from_millis(800));
        for period in [PERIOD, LEGACY_PERIOD] {
            let animation = motion(period);
            assert_eq!(animation.duration, period);
            assert!(!animation.oneshot);
            assert!(!animation.synced);
            assert_eq!(animation.max_fps, None);
            for phase in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
                assert_eq!((animation.easing)(phase).to_bits(), phase.to_bits());
            }
        }
    }
    #[test]
    fn invalid_numeric_sizes_are_refused_without_a_guessed_dimension() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(Points::try_from(px(value)), Err(InvalidPoints::NotFinite));
        }
        for value in [0.0, -0.0, -1.0, f32::MIN] {
            assert_eq!(Points::try_from(px(value)), Err(InvalidPoints::NotPositive));
        }
        for value in [f32::MIN_POSITIVE, 0.5, 10.0, 16.5, 36.0, f32::MAX] {
            assert!(Points::try_from(px(value)).is_ok());
        }
    }
}
