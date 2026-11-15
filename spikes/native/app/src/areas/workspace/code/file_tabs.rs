//! File presentation bindings use only ids allocated by the one layout owner.
//! Buffers, original attempts and admission remain in the existing file area;
//! this map does not allocate tabs, own docking state or resolve workspaces.
use super::{buffers::FileKey, file_open::FileOpenContext};
use crate::{areas::layout::registry::PanelContext, shell::Shell};
use std::collections::BTreeMap;
use vessel_app_kit::{
    Tokens,
    ui::{AnyElement, Context, IntoElement as _, Window},
};
use vessel_app_view::layout::{LayoutKey, Modified, TabId, Target};

pub(super) struct Binding {
    /// Original root receipt remains distinct from resource identity.
    pub(super) origin: FileOpenContext,
    /// Canonical source owner, never a route alias or cursor.
    pub(super) file: FileKey,
    acknowledged: Option<Modified>,
}
#[derive(Default)]
pub(super) struct FileTabs {
    bindings: BTreeMap<(LayoutKey, TabId), Binding>,
    close: Option<close::ClosePrompt>,
}
mod close;
mod close_view;
#[cfg(test)]
pub(crate) use close::FileCloseRequest;

impl FileTabs {
    pub(super) fn is_closing(&self, file: &FileKey) -> bool {
        self.close
            .as_ref()
            .is_some_and(|prompt| prompt.file == *file)
    }
    /// A visible or parked presentation is an owner reference, not disposable cache.
    pub(super) fn contains(&self, file: &FileKey) -> bool {
        self.bindings.values().any(|binding| binding.file == *file)
    }
    /// Path mutation admission sees existing resource references, not docking data.
    pub(super) fn files(&self) -> impl Iterator<Item = &FileKey> {
        self.bindings.values().map(|binding| &binding.file)
    }
    /// A compatibility alias/session cannot lend an old tab to a different node.
    #[cfg(test)]
    pub(super) fn conflicts(&self, workspace: &LayoutKey, file: &FileKey) -> bool {
        self.bindings.iter().any(|((at, _), binding)| {
            at == workspace
                && binding.file.root == file.root
                && binding.file.path == file.path
                && binding.file.node != file.node
        })
    }
    // Opening stays staged until the real UI caller and close dispatcher join it.
    #[cfg(test)]
    pub(super) fn can_bind(&self, workspace: &LayoutKey, file: &FileKey) -> bool {
        const MAX_FILE_BINDINGS: usize = 128;
        self.bindings.len() < MAX_FILE_BINDINGS
            || self
                .bindings
                .iter()
                .any(|((at, _), binding)| at == workspace && binding.file == *file)
    }
    /// Install only the actual id returned by the sole layout owner.
    #[cfg(test)]
    pub(super) fn bind(
        &mut self,
        workspace: LayoutKey,
        tab: TabId,
        origin: FileOpenContext,
        file: FileKey,
    ) {
        let _old = self.bindings.insert(
            (workspace, tab),
            Binding {
                origin,
                file,
                acknowledged: None,
            },
        );
    }
}

/// One main-registry renderer. Unbound/restored targets cannot manufacture a root receipt.
/// The current opener remains explicitly staged; no product caller can create a binding yet.
pub(crate) fn render_file_panel(
    shell: &Shell,
    panel: &PanelContext<'_>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let unavailable = || {
        super::frame::note(shell.words.layout().panel_unavailable().to_owned(), tokens)
            .into_any_element()
    };
    let Target::File { root, path } = panel.target else {
        return unavailable();
    };
    let Some(binding) = shell
        .areas
        .workspace
        .code
        .file_tabs
        .bindings
        .get(&(panel.workspace.clone(), panel.tab))
    else {
        return unavailable();
    };
    if shell
        .file_connection(binding.origin.scope().connection().machine().as_str())
        .is_some_and(|current| current.node() != binding.file.node)
    {
        return unavailable();
    }
    if binding.file.root != *root
        || binding.file.path != *path
        || binding.file.node != binding.origin.scope().node()
    {
        return unavailable();
    }
    let Some(viewer) = shell.areas.workspace.code.viewer_at(&binding.file) else {
        return unavailable();
    };
    if viewer.scope != *binding.origin.scope() {
        return unavailable();
    }
    use vessel_app_kit::ui::{ParentElement as _, Styled as _, div};
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(super::viewer::viewer_element(
            shell, viewer, tokens, window, cx,
        ))
        .children(close_view::dialog(
            shell,
            panel.workspace,
            panel.tab,
            tokens,
            window,
            cx,
        ))
        .into_any_element()
}

impl Shell {
    /// Publish the existing owner's readiness before any next effect can yield.
    /// The acknowledgement is only a write coalescer; close still checks the live owner.
    pub(super) fn publish_file_tabs(&mut self, file: &FileKey, cx: &mut Context<'_, Self>) {
        let panel = &self.areas.workspace.code;
        let modified = if panel.viewer_at(file).is_some_and(|viewer| {
            viewer.document_readiness() == vessel_app_view::desktop::quit::DocumentReadiness::Ready
        }) {
            Modified::Clean
        } else {
            Modified::Dirty
        };
        let pending: Vec<_> = panel
            .file_tabs
            .bindings
            .iter()
            .filter(|(_, binding)| binding.file == *file && binding.acknowledged != Some(modified))
            .map(|((workspace, tab), binding)| {
                (
                    workspace.clone(),
                    *tab,
                    Target::File {
                        root: binding.file.root,
                        path: binding.file.path.clone(),
                    },
                )
            })
            .collect();
        for (workspace, tab, expected) in pending {
            // Missing layout never releases its retained owner reference.
            if !self.layout_tab_matches(&workspace, tab, &expected) {
                continue;
            }
            let result = self.update_layout(
                &workspace,
                |layout| {
                    if layout
                        .tab(tab)
                        .is_none_or(|current| current.target() != &expected)
                    {
                        return Err(vessel_app_view::layout::LayoutError::InvalidState);
                    }
                    layout.set_modified(tab, modified)
                },
                cx,
            );
            if result.is_ok()
                && let Some(binding) = self
                    .areas
                    .workspace
                    .code
                    .file_tabs
                    .bindings
                    .get_mut(&(workspace, tab))
            {
                binding.acknowledged = Some(modified);
            }
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod close_tests;
