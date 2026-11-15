//! Wheel and recognized touch pans move the same bounded view viewport.
//! Fractional pixels accumulate so a trackpad does not jump a row per pixel.

use crate::shell::Shell;
use gpui::ScrollEvent;
use vessel_app_kit::terminal_grid::CellSize;
use vessel_app_kit::ui::{Context, Pixels};
use vessel_app_view::terminal::grid::Scroll;

#[derive(Debug, Default)]
/// Fractional gesture rows retained between pointer or touch events.
pub(super) struct Wheel(f32);

/// A single gesture cannot advance farther than the grid's wire count.
const MAX_GESTURE_ROWS: u16 = u16::MAX;

enum Step {
    Earlier(Scroll),
    Later(Scroll),
}

impl Wheel {
    #[expect(
        clippy::float_arithmetic,
        reason = "pixel-to-row accumulation at the rendering edge"
    )]
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "finite absolute whole rows are clamped before conversion; scrollback fits u32"
    )]
    fn step(&mut self, delta: Pixels, cell: CellSize) -> Option<Step> {
        let height = f32::from(cell.height);
        let pixels = f32::from(delta);
        if !pixels.is_finite() || !height.is_finite() || height <= 0.0 {
            return None;
        }
        self.0 = (self.0 + pixels / height)
            .clamp(-f32::from(MAX_GESTURE_ROWS), f32::from(MAX_GESTURE_ROWS));
        let whole = self.0.trunc();
        self.0 -= whole;
        let count = Scroll(whole.abs() as u32);
        if count.0 == 0 {
            None
        } else if whole > 0.0 {
            Some(Step::Earlier(count))
        } else {
            Some(Step::Later(count))
        }
    }
}

impl Shell {
    /// A wheel and a recognized touch pan address the same retained viewport.
    pub(super) fn terminal_scroll(
        &mut self,
        event: &ScrollEvent,
        cell: CellSize,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.terminal;
        if area.shown.is_none() {
            return;
        }
        // The frame remains on screen while parsing owns the model. Use
        // its retained range instead of dropping input during that handover.
        if let Some(model) = area.model.as_ref() {
            area.viewport.refresh(model.grid());
        }
        let delta = event.delta.pixel_delta(cell.height).y;
        match area.wheel.step(delta, cell) {
            Some(Step::Earlier(lines)) => area.viewport.earlier_retained(lines),
            Some(Step::Later(lines)) => area.viewport.later_retained(lines),
            None => return,
        }
        self.terminal_draw(cx);
        cx.stop_propagation();
    }
}
