//! Staged owner adapter: enable only together with the real canonical preference caller.
use super::ScrollbackLines;
use crate::shell::Shell;
use vessel_app_kit::ui::Context;

impl Shell {
    /// Settings owns persistence; terminal owns eviction and its in-flight model limit.
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
