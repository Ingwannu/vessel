//! Readonly display metadata stays with the same engine cut as its projected cells.

/// Program shape override; absence leaves each renderer's configured shape intact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    /// Occupy the current cell.
    Block,
    /// Mark the cell's baseline.
    Underline,
    /// Mark the insertion edge.
    Bar,
}
/// Program blink override is distinct from a renderer's reduced-motion policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorBlink {
    /// Blink when the renderer's motion policy permits it.
    On,
    /// Keep the program cursor steady.
    Off,
}
/// Independent optional overrides preserve an explicit reset to renderer defaults.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CursorAppearance {
    shape: Option<CursorShape>,
    blink: Option<CursorBlink>,
}
impl CursorAppearance {
    /// Decode closed wire choices without reconstructing a parser or inventing defaults.
    #[must_use]
    pub const fn new(shape: Option<CursorShape>, blink: Option<CursorBlink>) -> Self {
        Self { shape, blink }
    }
    /// Unset shape belongs to the receiving renderer's configured default.
    #[must_use]
    pub const fn shape(self) -> Option<CursorShape> {
        self.shape
    }
    /// Unset blink belongs to the receiving renderer's configured default.
    #[must_use]
    pub const fn blink(self) -> Option<CursorBlink> {
        self.blink
    }
    pub(crate) fn of(screen: &vt100::Screen) -> Self {
        match screen.cursor_style() {
            vt100::CursorStyle::Default => Self::default(),
            vt100::CursorStyle::BlinkBlock => {
                Self::new(Some(CursorShape::Block), Some(CursorBlink::On))
            }
            vt100::CursorStyle::SteadyBlock => {
                Self::new(Some(CursorShape::Block), Some(CursorBlink::Off))
            }
            vt100::CursorStyle::BlinkUnderline => {
                Self::new(Some(CursorShape::Underline), Some(CursorBlink::On))
            }
            vt100::CursorStyle::SteadyUnderline => {
                Self::new(Some(CursorShape::Underline), Some(CursorBlink::Off))
            }
            vt100::CursorStyle::BlinkVerticalBar => {
                Self::new(Some(CursorShape::Bar), Some(CursorBlink::On))
            }
            vt100::CursorStyle::SteadyVerticalBar => {
                Self::new(Some(CursorShape::Bar), Some(CursorBlink::Off))
            }
        }
    }
}
/// Whether the physical row continues onto the next row, not its predecessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowBreak {
    /// A selection crossing this row ending includes a line break.
    Hard,
    /// A selection crossing this row ending joins the next physical row.
    Soft,
}

#[cfg(test)]
mod tests;
