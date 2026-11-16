//! GPUI owns retained paint ranges; cursor dirtiness never invalidates static rows.
use super::{CellSize, GridFrame};
use crate::fonts::{FontStack, Typeface, active};
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, IntoElement as _, ParentElement as _,
    Render, Styled as _, Window, canvas, div,
};
use std::rc::Rc;

/// The terminal control owns retention; the application supplies immutable frame data.
#[derive(Debug)]
pub struct RetainedGrid {
    rows: Entity<Rows>,
    #[cfg(test)]
    probe: super::probe::DrawProbe,
}
impl RetainedGrid {
    /// Construct one control per terminal view, without application/session state.
    #[must_use]
    pub fn new(frame: &GridFrame, cell: CellSize, cx: &mut App) -> Self {
        #[cfg(test)]
        let probe = super::probe::DrawProbe::default();
        let rows = cx.new(|_cx| {
            Rows::new(
                frame,
                cell,
                active(Typeface::Code),
                #[cfg(test)]
                probe.clone(),
            )
        });
        Self {
            rows,
            #[cfg(test)]
            probe,
        }
    }
    /// Present current cursor/data; static row callbacks should survive cursor-only changes.
    pub fn draw(&mut self, frame: Rc<GridFrame>, cell: CellSize, cx: &mut App) -> AnyElement {
        self.draw_with_font(frame, cell, active(Typeface::Code), cx)
    }

    fn draw_with_font(
        &mut self,
        frame: Rc<GridFrame>,
        cell: CellSize,
        font: FontStack,
        cx: &mut App,
    ) -> AnyElement {
        if !self.rows.read(cx).matches(&frame, cell, &font) {
            // Notifications issued while rendering are observed on the NEXT draw.
            // A fresh immutable child gives this draw a fresh GPUI cache identity;
            // cursor-only frames keep the same child and retained paint ranges.
            self.rows = cx.new(|_cx| {
                Rows::new(
                    &frame,
                    cell,
                    font,
                    #[cfg(test)]
                    self.probe.clone(),
                )
            });
        }
        // This aggregate stays uncached: refreshing a dirty cached parent would
        // force GPUI to refresh its otherwise unchanged cached children as well.
        #[cfg(test)]
        let cursor_probe = self.probe.clone();
        div()
            .size_full()
            .relative()
            .child(crate::cached_view(&self.rows))
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, (), window, _cx| {
                        #[cfg(test)]
                        cursor_probe.note(super::probe::Stage::Cursor);
                        super::paint_cursor(frame.cursor, cell, bounds, window);
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .into_any_element()
    }
}

fn static_frame(frame: &GridFrame) -> Rc<GridFrame> {
    Rc::new(GridFrame {
        cursor: None,
        ..frame.clone()
    })
}

#[derive(Debug)]
struct Rows {
    frame: Rc<GridFrame>,
    cell: CellSize,
    font: FontStack,
    #[cfg(test)]
    probe: super::probe::DrawProbe,
}
impl Rows {
    fn new(
        frame: &GridFrame,
        cell: CellSize,
        font: FontStack,
        #[cfg(test)] probe: super::probe::DrawProbe,
    ) -> Self {
        Self {
            frame: static_frame(frame),
            cell,
            font,
            #[cfg(test)]
            probe,
        }
    }
    fn matches(&self, frame: &GridFrame, cell: CellSize, font: &FontStack) -> bool {
        Rc::ptr_eq(&self.frame.rows, &frame.rows)
            && self.frame.background == frame.background
            && self.frame.font_size == frame.font_size
            && self.cell == cell
            && &self.font == font
    }
}
impl Render for Rows {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        super::control(
            Rc::clone(&self.frame),
            self.cell,
            self.font.clone(),
            #[cfg(test)]
            Some(self.probe.clone()),
        )
    }
}
#[cfg(test)]
mod tests;
