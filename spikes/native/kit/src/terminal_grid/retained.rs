//! Stable control admission, initially delegating to the existing monolithic renderer.
use super::{CellSize, GridFrame};
use gpui::{AnyElement, App, IntoElement as _};
use std::rc::Rc;

/// The terminal control owns retention; the application supplies immutable frame data.
pub struct RetainedGrid {
    #[cfg(test)]
    probe: super::probe::DrawProbe,
}
impl RetainedGrid {
    /// Construct one control per terminal view, without application/session state.
    #[must_use]
    pub fn new(_frame: Rc<GridFrame>, _cell: CellSize, _cx: &mut App) -> Self {
        Self {
            #[cfg(test)]
            probe: super::probe::DrawProbe::default(),
        }
    }
    /// Present current cursor/data; static row callbacks should survive cursor-only changes.
    pub fn draw(&mut self, frame: Rc<GridFrame>, cell: CellSize, _cx: &mut App) -> AnyElement {
        super::control(
            frame,
            cell,
            #[cfg(test)]
            Some(self.probe.clone()),
        )
        .into_any_element()
    }
}
#[cfg(test)]
mod tests;
