//! Read-only owner queries never manufacture an absent layout or resource.
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::layout::{Layout, LayoutKey, LayoutMode, PaneId, TabId, Target};

/// Drawing and polling share one effective projection. A removed maximized pane
/// falls back to the full tree instead of hiding every surviving panel.
pub(super) fn projected_pane(
    shell: &Shell,
    layout: &Layout,
    workspace: &LayoutKey,
    window: &Window,
) -> Option<PaneId> {
    if super::mode(window) == LayoutMode::Compact {
        return Some(layout.focused_pane());
    }
    shell
        .areas
        .layout
        .maximized
        .as_ref()
        .filter(|(key, _)| key == workspace)
        .map(|(_, pane)| *pane)
        .filter(|id| {
            layout
                .root()
                .panes()
                .into_iter()
                .any(|pane| pane.id() == *id)
        })
}

impl Shell {
    /// A retained tab id can be retargeted. Resource callbacks check their
    /// captured target too; existence/visibility alone cannot bless old actions.
    /// This reads an existing entry and grants no connection or resource scope.
    pub(crate) fn layout_tab_matches(
        &self,
        workspace: &LayoutKey,
        tab: TabId,
        target: &Target,
    ) -> bool {
        super::initialized(self);
        self.areas
            .layout
            .book
            .borrow()
            .as_ref()
            .and_then(|book| book.get(workspace))
            .and_then(|layout| layout.tab(tab))
            .is_some_and(|current| current.target() == target)
    }
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
        let projection = projected_pane(self, layout, workspace, window);
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

#[cfg(test)]
mod tests;
