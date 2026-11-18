//! Validated snap heights preserve the shared engine's existing 60/90 defaults.

use super::motion::Stop;

const FULL_PERCENT: u8 = 100;

/// A positive percentage of a viewport, bounded at its full height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Percentage(u8);

impl Percentage {
    /// The complete viewport is also a safe fallback for authored configuration.
    pub const FULL: Self = Self(FULL_PERCENT);
    /// Parse an authored height once, before it enters layout or gesture math.
    #[must_use]
    pub const fn new(value: u8) -> Option<Self> {
        if value > 0 && value <= FULL_PERCENT {
            Some(Self(value))
        } else {
            None
        }
    }

    /// The authored integer percentage (for configuration inspection).
    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }

    #[expect(
        clippy::float_arithmetic,
        reason = "validated percentages convert to finite viewport fractions"
    )]
    pub(crate) fn fraction(self) -> f32 {
        f32::from(self.0) / f32::from(FULL_PERCENT)
    }

    #[expect(
        clippy::float_arithmetic,
        reason = "normalized top offsets retain correctly rounded default stop values"
    )]
    fn top(self) -> f32 {
        f32::from(FULL_PERCENT.saturating_sub(self.0)) / f32::from(FULL_PERCENT)
    }
}

/// Exactly two ordered open heights; closed remains the viewport's bottom edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapPoints {
    compact: Percentage,
    expanded: Percentage,
}

impl Default for SnapPoints {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl SnapPoints {
    pub(crate) const DEFAULT: Self = Self {
        compact: Percentage(60),
        expanded: Percentage(90),
    };

    /// A content-sized sheet (Paseo's `enableDynamicSizing`): positions are
    /// measured against its own height, both open stops at all of it, so
    /// one open stop and closed (`host::SheetState::frame_content`). Not
    /// reachable through [`Self::new`], which keeps authored stops apart.
    pub(crate) const CONTENT: Self = Self {
        compact: Percentage::FULL,
        expanded: Percentage::FULL,
    };

    /// Refuse reversed/equal stops and percentages outside the viewport.
    #[must_use]
    pub const fn new(compact: u8, expanded: u8) -> Option<Self> {
        match (Percentage::new(compact), Percentage::new(expanded)) {
            (Some(compact), Some(expanded)) if compact.0 < expanded.0 => {
                Some(Self { compact, expanded })
            }
            (Some(_) | None, Some(_) | None) => None,
        }
    }

    /// Initial open height.
    #[must_use]
    pub const fn compact(self) -> Percentage {
        self.compact
    }
    /// Expanded open height and content-scroll handoff boundary.
    #[must_use]
    pub const fn expanded(self) -> Percentage {
        self.expanded
    }

    pub(super) fn position(self, stop: Stop) -> f32 {
        match stop {
            Stop::Closed => 1.0,
            Stop::Compact => self.compact.top(),
            Stop::Expanded => self.expanded.top(),
        }
    }
}
