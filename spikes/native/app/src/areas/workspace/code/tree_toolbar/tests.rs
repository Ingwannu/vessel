//! Toolbar intent retains the root receipt drawn before queued UI delivery.
use super::*;
use crate::areas::workspace::code::{Tab, actions::EditKind, io_tests};

fn opening(shell: &mut Shell) -> FileOpenContext {
    io_tests::target(shell, io_tests::root());
    let target = shell.areas.workspace.code.target.as_ref().unwrap();
    let context = target.admission.as_ref().unwrap().context.clone();
    shell.areas.session.open = Some(crate::areas::session::Open {
        machine: "node".into(),
        session: target.session,
        title: "fixture".into(),
        agent: "fixture".into(),
        structured: true,
        live: true,
    });
    let mut machine = vessel_app_view::Machine::unreachable("node".into(), "fixture");
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    shell.fleet.machines.push(machine);
    shell.route = crate::routes::Route::Session;
    shell.areas.workspace.code.open = true;
    shell.areas.workspace.code.tab = Tab::Files;
    shell.file_open_context().unwrap()
}

#[gpui::test]
fn new_entry_does_not_restamp_an_old_toolbar_after_root_refresh(cx: &mut gpui::TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let old = opening(shell);
        let receipt = shell.next_file_ticket().unwrap();
        shell
            .areas
            .workspace
            .code
            .target
            .as_mut()
            .unwrap()
            .admission
            .as_mut()
            .unwrap()
            .ticket = receipt;
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
    });
}
