//! Staged under A10: E2 preference wiring removes cfg(test) from this module.
//! One retained preference applies to both idle models and executor handovers.
use super::ScrollbackLines;
use crate::shell::Shell;
use vessel_app_kit::ui::Context;

impl Shell {
    /// Settings owns persistence; the terminal owns live eviction and viewport state.
    /// Retain a change while replay owns the model, then apply it before drawing.
    pub(crate) fn terminal_scrollback_changed(
        &mut self,
        limit: ScrollbackLines,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.terminal;
        area.scrollback = limit;
        if let Some(model) = area.model.as_mut() {
            model.set_scrollback(limit);
        }
        self.terminal_draw(cx);
    }
}
