//! Queued real form/menu callbacks cannot consume another root or opening.
use super::actions::Action;
use super::io_tests::{root, target};
use crate::shell::Shell;
use gpui::{TestAppContext, point, px};

#[gpui::test]
fn old_menu_and_delete_answers_cannot_consume_the_new_opening(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let a = root();
        target(shell, a);
        shell.open_file_actions(
            "victim".parse().unwrap(),
            false,
            true,
            point(px(0.0), px(0.0)),
            window,
            cx,
        );
        let old = shell
            .areas
            .workspace
            .code
            .files
            .actions
            .target
            .as_ref()
            .unwrap()
            .opening
            .ticket;
        shell.file_action(old, Action::Delete, window, cx);
        let b = root();
        target(shell, b);
        shell.confirm_delete(old, cx);
        assert!(shell.areas.workspace.code.mutation.is_none());
        shell.open_file_actions(
            "victim".parse().unwrap(),
            false,
            true,
            point(px(0.0), px(0.0)),
            window,
            cx,
        );
        let current = shell
            .areas
            .workspace
            .code
            .files
            .actions
            .target
            .as_ref()
            .unwrap()
            .opening
            .ticket;
        shell.file_action(old, Action::Delete, window, cx);
        assert_eq!(
            shell
                .areas
                .workspace
                .code
                .files
                .actions
                .target
                .as_ref()
                .unwrap()
                .opening
                .ticket,
            current
        );
        shell.file_action(current, Action::Delete, window, cx);
        shell.confirm_delete(old, cx);
        shell.cancel_delete(old, cx);
        assert_eq!(
            shell
                .areas
                .workspace
                .code
                .files
                .actions
                .deleting
                .as_ref()
                .unwrap()
                .opening
                .ticket,
            current
        );
        assert!(shell.areas.workspace.code.mutation.is_none());
        shell.confirm_delete(current, cx);
        assert!(shell.areas.workspace.code.mutation.is_some());
        let key = |root| {
            let scope = shell.areas.workspace.code.scope().unwrap();
            let scope = vessel_app_view::workspace::file_scope::FileScope::new(
                scope.connection().clone(),
                root,
            );
            super::buffers::FileKey::new(&scope, "victim".parse().unwrap())
        };
        assert!(shell.areas.workspace.code.file_mutating(&key(b)));
        assert!(!shell.areas.workspace.code.file_mutating(&key(a)));
    });
}

#[gpui::test]
fn old_enter_and_cancel_cannot_submit_or_clear_a_new_root_form(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        target(shell, root());
        shell.open_file_actions(
            "".parse().unwrap(),
            true,
            true,
            point(px(0.0), px(0.0)),
            window,
            cx,
        );
        let old = shell
            .areas
            .workspace
            .code
            .files
            .actions
            .target
            .as_ref()
            .unwrap()
            .opening
            .ticket;
        shell.file_action(old, Action::NewFile, window, cx);
        target(shell, root());
        shell.finish_edit(old, "outside.txt", cx);
        assert!(shell.areas.workspace.code.mutation.is_none());
        shell.open_file_actions(
            "".parse().unwrap(),
            true,
            true,
            point(px(0.0), px(0.0)),
            window,
            cx,
        );
        let current = shell
            .areas
            .workspace
            .code
            .files
            .actions
            .target
            .as_ref()
            .unwrap()
            .opening
            .ticket;
        shell.file_action(current, Action::NewFile, window, cx);
        shell.finish_edit(old, "old.txt", cx);
        shell.cancel_edit(old, cx);
        assert_eq!(
            shell
                .areas
                .workspace
                .code
                .files
                .actions
                .editing
                .as_ref()
                .unwrap()
                .opening
                .ticket,
            current
        );
        assert!(shell.areas.workspace.code.mutation.is_none());
        shell.finish_edit(current, "current.txt", cx);
        assert!(shell.areas.workspace.code.mutation.is_some());
    });
}
