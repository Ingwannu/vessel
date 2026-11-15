//! Read-only owner queries never manufacture an absent layout or resource.
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::layout::{LayoutKey, LayoutMode, TabId};

impl Shell {
    /// Existence is independent of navigation and does not authorize resource I/O.
    pub(crate) fn layout_tab_exists(&self, workspace: &LayoutKey, tab: TabId) -> bool {
        super::initialized(self);
        self.areas
            .layout
            .book
            .borrow()
            .as_ref()
            .and_then(|book| book.get(workspace))
            .is_some_and(|layout| layout.tab(tab).is_some())
    }
    /// The caller additionally checks OS window visibility before polling.
    /// Window bounds are required because compact mode projects only one pane.
    pub(crate) fn layout_panel_visible(
        &self,
        workspace: &LayoutKey,
        tab: TabId,
        window: &Window,
    ) -> bool {
        if self.route != crate::routes::Route::Session || super::key(self).as_ref() != Ok(workspace)
        {
            return false;
        }
        super::initialized(self);
        let book = self.areas.layout.book.borrow();
        let Some(layout) = book.as_ref().and_then(|book| book.get(workspace)) else {
            return false;
        };
        if let Some(sheet) = layout.sheets().last() {
            return sheet.tab == tab;
        }
        let projection = if super::mode(window) == LayoutMode::Compact {
            Some(layout.focused_pane())
        } else {
            self.areas
                .layout
                .maximized
                .as_ref()
                .filter(|(key, _)| key == workspace)
                .map(|(_, pane)| *pane)
        };
        layout
            .root()
            .panes()
            .into_iter()
            .any(|pane| projection.is_none_or(|id| pane.id() == id) && pane.focused() == Some(tab))
    }
    /// Sidebar actions exit focus mode idempotently, never toggle into it.
    pub(crate) fn layout_exit_focus(&mut self, cx: &mut Context<'_, Self>) {
        let Ok(workspace) = super::key(self) else {
            return;
        };
        if self
            .areas
            .layout
            .maximized
            .as_ref()
            .is_some_and(|(key, _)| *key == workspace)
        {
            self.areas.layout.maximized = None;
            cx.notify();
        }
    }
}
