//! The layout area controls placement; this adapter keeps filesystem
//! ownership and request effects inside the existing code area.

use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{AnyElement, Context, IntoElement as _, Window};

use super::{Place, Tab};
use crate::shell::Shell;

impl Shell {
    /// Reuse an already resolved session root only for its exact node and
    /// session. A composer may resolve session.get itself when absent.
    pub(crate) fn code_workspace_scope(
        &self,
        machine: &vessel_app_view::machine_book::Name,
        session: vessel_proto::SessionId,
    ) -> Option<vessel_proto::workspace::WorkspaceRoot> {
        let panel = &self.areas.workspace.code;
        let target = panel.target.as_ref()?;
        if target.machine != machine.as_str() || target.session != session {
            return None;
        }
        panel.root().map(|(_, root)| *root)
    }
}

/// Explorer singleton selection shared with the layout owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExplorerKind {
    /// File tree and source buffers.
    Files,
    /// Git's working changes.
    Changes,
}

/// Reveal the requested explorer without creating another file owner.
pub(crate) fn show_layout_explorer(
    shell: &mut Shell,
    kind: ExplorerKind,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    shell.areas.workspace.code.open = true;
    shell.areas.workspace.code.files.options = shell.prefs.explorer;
    shell.search_words(window, cx);
    shell.code_follow();
    // Restore only the shown root's retained keys; this operation performs no I/O.
    shell.restore_file_tree();
    let tab = match kind {
        ExplorerKind::Files => Tab::Files,
        ExplorerKind::Changes => Tab::Changes,
    };
    shell.code_tab(tab, cx);
}

/// Draw only the area body; panes own its frame, tabs and placement.
pub(crate) fn render_explorer(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let labels = shell.words().labels();
    match shell
        .areas
        .workspace
        .code
        .target
        .as_ref()
        .map(|target| &target.place)
    {
        None | Some(Place::Reading) => {
            super::frame::note(labels.loading, tokens).into_any_element()
        }
        Some(Place::Outside) => {
            super::frame::note(labels.not_in_project, tokens).into_any_element()
        }
        Some(Place::Failed(why)) => super::frame::note(why.clone(), tokens).into_any_element(),
        Some(Place::Root(_)) => match shell.areas.workspace.code.tab {
            Tab::Changes => super::changes::render(shell, tokens, window, cx),
            Tab::Files => super::files::render(shell, tokens, window, cx),
        },
    }
}
