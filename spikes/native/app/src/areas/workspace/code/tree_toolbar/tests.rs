//! Toolbar intent retains the root receipt drawn before queued UI delivery.
use super::*;
use crate::areas::workspace::code::{Tab, actions::EditKind, io_tests};

fn opening(
    shell: &mut Shell,
    scene: &mut vessel_app_link::testing::Scenario,
    cx: &mut Context<'_, Shell>,
) -> FileOpenContext {
    let root = io_tests::root();
    let opening = super::super::root_fixture::acquire(shell, scene, root, cx);
    shell.areas.workspace.code.open = true;
    shell.areas.workspace.code.tab = Tab::Files;
    opening
}

#[gpui::test]
fn new_entry_does_not_restamp_an_old_toolbar_after_root_refresh(cx: &mut gpui::TestAppContext) {
    let (shell, cx, mut scene) = super::super::root_fixture::setup(cx, "node");
    shell.update_in(cx, |shell, window, cx| {
        let old = opening(shell, &mut scene, cx);
        let _refreshed =
            super::super::root_fixture::refresh(shell, &mut scene, old.scope().root(), cx);
        for action in [Action::NewFile, Action::NewFolder] {
            shell.tree_new(&old, action, window, cx);
            let actions = &shell.areas.workspace.code.files.actions;
            assert!(actions.editing.is_none());
            assert!(actions.target.is_none());
        }
        let fresh = shell.file_open_context().unwrap();
        shell.tree_new(&fresh, Action::NewFolder, window, cx);
        let actions = &mut shell.areas.workspace.code.files.actions;
        let editing = actions.editing.take().unwrap();
        assert_eq!(editing.opening.scope, *fresh.scope());
        assert_eq!(editing.kind, EditKind::NewFolder(RelPath::root()));
        shell.areas.workspace.code.open = false;
        shell.tree_new(&fresh, Action::NewFile, window, cx);
        assert!(shell.areas.workspace.code.files.actions.editing.is_none());
        scene.close(shell.fixture_link()).unwrap();
    });
}
