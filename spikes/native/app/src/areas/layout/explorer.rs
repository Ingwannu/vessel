//! Ephemeral Explorer projection from the sole book, not another dock or scope.
use super::registry::{PanelContext, PanelFocus};
use crate::shell::Shell;
use vessel_app_view::layout::{ExplorerVisibility, PaneId};

impl Shell {
    /// Supply the actual selected Explorer tab after releasing the book borrow.
    /// The host supplies focus; owners still require their original resource and
    /// connection admission. Hidden docks and a covering sheet invoke no body.
    pub(crate) fn layout_with_explorer<R>(
        &self,
        focus: PanelFocus,
        render: impl FnOnce(&PanelContext<'_>) -> R,
    ) -> Option<R> {
        if self.route != crate::routes::Route::Session {
            return None;
        }
        let workspace = super::key(self).ok()?;
        super::initialized(self);
        let tab = {
            let book = self.areas.layout.book.borrow();
            let layout = book.as_ref()?.get(&workspace)?;
            if layout.explorer_visibility() != ExplorerVisibility::Shown
                || !layout.sheets().is_empty()
            {
                return None;
            }
            layout.tab(layout.explorer().focused()?)?.clone()
        };
        // Only small presentation metadata is copied. Rendering may consult the
        // book again; no RefCell guard or resource object crosses that callback.
        Some(render(&PanelContext {
            workspace: &workspace,
            pane: PaneId::EXPLORER,
            tab: tab.id(),
            target: tab.target(),
            focus,
        }))
    }
}

#[cfg(test)]
mod tests;
