//! The layout area controls placement; this adapter keeps filesystem
//! ownership and request effects inside the existing code area.

use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{AnyElement, Context, IntoElement as _, Window};

use super::{Place, Tab};
use crate::shell::Shell;

impl Shell {
    /// Snapshot of the active session's already-resolved file root. This is
    /// request scope, not a live connection lease; G4 owns effect admission.
    pub(crate) fn file_workspace_scope(
        &self,
    ) -> Option<(
        vessel_app_view::machine_book::Name,
        vessel_proto::workspace::WorkspaceRoot,
    )> {
        let (machine, session) = self.code_session()?;
        let machine = vessel_app_view::machine_book::Name::parse(&machine).ok()?;
        let root = self.code_workspace_scope(&machine, session)?;
        Some((machine, root))
    }

    /// Capture the active session's already resolved original file admission.
    /// This supplies request facts only: G4 rechecks every actual effect, and
    /// this is not a runtime owning-workspace or file-pane resolver.
    pub(crate) fn active_file_scope(
        &self,
    ) -> Option<vessel_app_view::workspace::file_scope::FileScope> {
        let (machine, session) = self.code_session()?;
        let target = self.areas.workspace.code.target.as_ref()?;
        if target.machine != machine || target.session != session {
            return None;
        }
        self.file_admitted_scope()
    }

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
        self.file_admitted_scope().map(|scope| scope.root())
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
