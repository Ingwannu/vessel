//! Mounted control identity from the canonical owner token and captured claim.
//! Identity grants no input, follow or transport authority.
use std::sync::Arc;
use vessel_app_kit::ui::ElementId;
#[cfg(test)]
use vessel_app_view::layout::LayoutKey;
#[cfg(test)]
use vessel_app_view::residents::SessionContext;

#[derive(Debug, Clone)]
pub(super) struct RenderScope(ElementId);

impl RenderScope {
    /// A token display is a lossless owner-provided identity, never authority.
    #[cfg(test)]
    pub(super) fn admitted(context: &SessionContext) -> Self {
        let resource = child(
            ElementId::Name("conversation-render".into()),
            context.resident.to_string(),
        );
        let claim = match &context.claim.workspace {
            LayoutKey::Workspace { node, workspace } => child(
                child(child(resource, "workspace"), node.to_string()),
                workspace.to_string(),
            ),
            LayoutKey::Session { machine, session } => child(
                child(
                    child(resource, "session-bridge"),
                    machine.as_str().to_owned(),
                ),
                session.to_string(),
            ),
        };
        Self(child(
            child(claim, "tab"),
            context.claim.tab.get().to_string(),
        ))
    }

    pub(super) fn origin(&self) -> Self {
        Self(child(self.0.clone(), "origin"))
    }

    pub(super) fn sheet(&self) -> Self {
        Self(child(self.0.clone(), "sheet"))
    }

    pub(super) fn with<T>(
        &self,
        window: &mut vessel_app_kit::ui::Window,
        draw: impl FnOnce(&mut vessel_app_kit::ui::Window) -> T,
    ) -> T {
        window.with_id(self.element(), draw)
    }

    pub(super) fn element(&self) -> ElementId {
        self.0.clone()
    }
}

fn child(parent: ElementId, label: impl Into<vessel_app_kit::ui::SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(parent), label.into())
}

#[cfg(test)]
mod tests;
