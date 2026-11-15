//! Authored modal geometry opts into the existing renderer; legacy sizes stay fixed.

use super::DialogSize;
use crate::sheet::snap::{Percentage, SnapPoints};
use std::num::NonZeroU16;

const NORMAL_WIDTH: u16 = 520;
const WIDE_WIDTH: u16 = 640;
const DESKTOP_PERCENT: u8 = 85;
const COMPACT_PERCENT: u8 = 90;

/// A positive logical-pixel width, distinct from viewport percentages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogWidth(NonZeroU16);

impl DialogWidth {
    /// Refuse a zero maximum before it reaches the renderer.
    #[must_use]
    pub const fn new(value: u16) -> Option<Self> {
        match NonZeroU16::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
    /// The authored logical-pixel count, for configuration inspection.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0.get()
    }
    pub(crate) fn pixels(self) -> gpui::Pixels {
        gpui::px(f32::from(self.get()))
    }
}

/// A fixed fitting sheet or an explicit pair of draggable viewport heights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactLayout {
    /// Natural content height, capped at this viewport percentage.
    Fixed(Percentage),
    /// Read-only scrollable content shares the snap engine's gesture handoff.
    Draggable(SnapPoints),
}

/// Fitting dialogs and fixed-height release sheets have different desktop rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopHeight {
    /// Preserve natural fitting height below the percentage cap.
    AtMost(Percentage),
    /// Use the authored percentage even when the content is short.
    Exact(Percentage),
}

/// Shared forms already own row insets; ordinary dialogs keep their body inset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodySpacing {
    /// Preserve the existing 24px body inset.
    Inset,
    /// The caller body supplies its own horizontal and vertical spacing.
    Caller,
}

/// Positive desktop maximum width and validated viewport-height limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogLayout {
    width: DialogWidth,
    height: DesktopHeight,
    compact: CompactLayout,
    body_spacing: BodySpacing,
}

impl DialogLayout {
    /// Refuse zero width before rendering. Height and compact values have
    /// already passed their own validating constructors.
    #[must_use]
    pub const fn new(width: u16, height: DesktopHeight, compact: CompactLayout) -> Option<Self> {
        if let Some(width) = DialogWidth::new(width) {
            Some(Self {
                width,
                height,
                compact,
                body_spacing: BodySpacing::Inset,
            })
        } else {
            None
        }
    }

    /// The desktop maximum in logical pixels.
    #[must_use]
    pub const fn width(self) -> DialogWidth {
        self.width
    }
    /// The desktop viewport-height limit.
    #[must_use]
    pub const fn height(self) -> DesktopHeight {
        self.height
    }
    /// Fixed or draggable compact presentation.
    #[must_use]
    pub const fn compact(self) -> CompactLayout {
        self.compact
    }

    /// Select caller-owned form spacing without another compact renderer.
    #[must_use]
    pub const fn with_body_spacing(self, body_spacing: BodySpacing) -> Self {
        Self {
            body_spacing,
            ..self
        }
    }

    pub(super) const fn body_spacing(self) -> BodySpacing {
        self.body_spacing
    }
}

impl DialogSize {
    pub(super) const fn layout(self) -> DialogLayout {
        match self {
            Self::Normal => legacy(NORMAL_WIDTH),
            Self::Wide => legacy(WIDE_WIDTH),
            Self::Configured(layout) => layout,
        }
    }
}

const fn legacy(width: u16) -> DialogLayout {
    // These pre-existing defaults are validated constants, not external input.
    let height = match Percentage::new(DESKTOP_PERCENT) {
        Some(value) => value,
        None => Percentage::FULL,
    };
    let compact = match Percentage::new(COMPACT_PERCENT) {
        Some(value) => value,
        None => Percentage::FULL,
    };
    let width = match DialogWidth::new(width) {
        Some(width) => width,
        None => DialogWidth(NonZeroU16::MIN),
    };
    DialogLayout {
        width,
        height: DesktopHeight::AtMost(height),
        compact: CompactLayout::Fixed(compact),
        body_spacing: BodySpacing::Inset,
    }
}
