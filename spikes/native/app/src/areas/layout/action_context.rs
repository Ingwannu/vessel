//! Captured palette commands never dispatch through a different workspace.
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::layout::{LayoutAction, LayoutActionContext, LayoutError};

impl Shell {
    pub(crate) fn layout_action_context(&self) -> Option<LayoutActionContext> {
        if self.route != crate::routes::Route::Session || self.areas.session.open.is_none() {
            return None;
        }
        let key = super::key(self).ok()?;
        super::with_layout_at(self, &key, |layout| Ok(layout.action_context(&key))).ok()
    }
    pub(crate) fn layout_action_context_is_current(&self, context: &LayoutActionContext) -> bool {
        self.layout_action_context().as_ref() == Some(context)
    }
    /// Stale actions remain handled so a refused close cannot fall through to
    /// the lower-priority desktop window-close command.
    pub(crate) fn run_layout_action_at(
        &mut self,
        context: &LayoutActionContext,
        action: LayoutAction,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if !self.layout_action_context_is_current(context) {
            self.layout_result(Err(LayoutError::InvalidState), cx);
            return true;
        }
        self.run_layout_action(action, window, cx)
    }
}
