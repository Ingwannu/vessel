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
        let shape = match screen.cursor_style() {
            vt100::CursorStyle::Default => None,
            vt100::CursorStyle::BlinkBlock | vt100::CursorStyle::SteadyBlock => {
                Some(CursorShape::Block)
            }
            vt100::CursorStyle::BlinkUnderline | vt100::CursorStyle::SteadyUnderline => {
                Some(CursorShape::Underline)
            }
            vt100::CursorStyle::BlinkVerticalBar | vt100::CursorStyle::SteadyVerticalBar => {
                Some(CursorShape::Bar)
            }
        };
        let blink = screen.cursor_blink_override().map(|blink| match blink {
            vt100::CursorBlink::On => CursorBlink::On,
            vt100::CursorBlink::Off => CursorBlink::Off,
        });
        Self::new(shape, blink)
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
