//! Mutations cannot discard dirty retained buffers or apply an old
//! workspace's acknowledgement to the current workspace's form/viewer.

use super::super::io_tests;
use super::*;
use gpui::TestAppContext;
use vessel_proto::workspace::{DeletePath, RenamePath};

fn load(
    shell: &mut Shell,
    root: vessel_proto::workspace::WorkspaceRoot,
    path: &str,
    cx: &mut Context<'_, Shell>,
) -> FileKey {
    shell.open_file_at(path.parse().unwrap(), None, cx);
    let key = shell.areas.workspace.code.file_key().unwrap();
    let ticket = shell.editing_for(&key).unwrap().reading.unwrap().0;
    shell.editor_read_answered(
        "node",
        ticket,
        &super::super::editing::whole_read(root, key.path.clone()),
        Ok(io_tests::text("saved")),
        cx,
    );
    key
}

#[gpui::test]
fn dirty_or_pending_descendant_refuses_directory_mutation(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = io_tests::root();
        io_tests::target(shell, root);
        let dirty = load(shell, root, "src/a.rs", cx);
        let _step = shell.editing_for(&dirty).unwrap().model.edit("unsaved");
        shell.open_file_at("other.rs".parse().unwrap(), None, cx);
        // Opening another file flushes but must still retain this write.
        let rename = PathChange::Rename(RenamePath {
            root,
            from: "src".parse().unwrap(),
            to: "lib".parse().unwrap(),
        });
        shell.begin_file_mutation("node", rename, cx);
        assert!(shell.areas.workspace.code.mutation.is_none());
        assert_eq!(
            shell.editing_for(&dirty).unwrap().model.content(),
            "unsaved"
        );
        assert!(shell.editing_for(&dirty).unwrap().writing.is_some());
        assert!(shell.areas.workspace.code.files.actions.refused.is_some());
    });
}

#[gpui::test]
fn rename_rekeys_clean_descendants_only_after_matching_ack(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = io_tests::root();
        io_tests::target(shell, root);
        let before = load(shell, root, "src/a.rs", cx);
        let sibling = load(shell, root, "src-old/a.rs", cx);
        let _expanded = shell
            .areas
            .workspace
            .code
            .files
            .open
            .insert("src/sub".parse().unwrap());
        let change = PathChange::Rename(RenamePath {
            root,
            from: "src".parse().unwrap(),
            to: "lib".parse().unwrap(),
        });
        shell.begin_file_mutation("node", change.clone(), cx);
        let ticket = shell.areas.workspace.code.mutation.as_ref().unwrap().ticket;
        shell.file_mutation_answered("other", &change, ticket, Ok(()), cx);
        assert!(shell.areas.workspace.code.mutation.is_some());
        assert!(shell.editing_for(&before).is_some());
        shell.file_mutation_answered("node", &change, ticket, Ok(()), cx);
        assert!(shell.areas.workspace.code.mutation.is_none());
        assert!(shell.editing_for(&before).is_none());
        let after = FileKey {
            path: "lib/a.rs".parse().unwrap(),
            ..before
        };
        assert_eq!(shell.editing_for(&after).unwrap().model.content(), "saved");
        assert_eq!(shell.areas.workspace.code.file_key(), Some(sibling));
        assert!(
            shell
                .areas
                .workspace
                .code
                .files
                .open
                .contains(&"lib/sub".parse().unwrap())
        );
        shell.file_mutation_answered("node", &change, ticket, Ok(()), cx);
        assert!(shell.editing_for(&after).is_some());
    });
}

#[gpui::test]
fn old_root_delete_settles_its_resources_without_closing_current_file(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let old = io_tests::root();
        io_tests::target(shell, old);
        let deleted = load(shell, old, "a.txt", cx);
        let change = PathChange::Delete(DeletePath {
            root: old,
            path: deleted.path.clone(),
            recursive: false,
        });
        shell.begin_file_mutation("node", change.clone(), cx);
        let ticket = shell.areas.workspace.code.mutation.as_ref().unwrap().ticket;
        assert!(shell.park_file());
        let new = io_tests::root();
        io_tests::target(shell, new);
        let active = load(shell, new, "a.txt", cx);
        shell.file_mutation_answered("node", &change, ticket, Ok(()), cx);
        assert!(shell.editing_for(&deleted).is_none());
        assert_eq!(shell.areas.workspace.code.file_key(), Some(active.clone()));
        assert_eq!(shell.editing_for(&active).unwrap().model.content(), "saved");
    });
}

#[gpui::test]
fn directory_rename_preserves_deep_clean_retained_descendants(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = io_tests::root();
        io_tests::target(shell, root);
        let before = load(shell, root, "src/nested/deeper/a.rs", cx);
        let other = load(shell, root, "other.rs", cx);
        let change = PathChange::Rename(RenamePath {
            root,
            from: "src".parse().unwrap(),
            to: "lib".parse().unwrap(),
        });
        shell.begin_file_mutation("node", change.clone(), cx);
        let ticket = shell.areas.workspace.code.mutation.as_ref().unwrap().ticket;
        shell.file_mutation_answered("node", &change, ticket, Ok(()), cx);
        let after = FileKey {
            path: "lib/nested/deeper/a.rs".parse().unwrap(),
            ..before.clone()
        };
        assert!(shell.editing_for(&before).is_none());
        assert_eq!(shell.editing_for(&after).unwrap().model.content(), "saved");
        assert_eq!(shell.areas.workspace.code.file_key(), Some(other));
    });
}

#[gpui::test]
fn selected_actor_without_network_dispatch_does_not_leave_a_phantom_path_hold(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = io_tests::root();
        io_tests::target(shell, root);
        let key = load(shell, root, "before.txt", cx);
        let change = PathChange::Rename(RenamePath {
            root,
            from: key.path.clone(),
            to: "after.txt".parse().unwrap(),
        });
        for problem in [
            Problem::Unsent,
            Problem::Older,
            Problem::Node {
                code: vessel_proto::ErrorCode::Forbidden,
                message: String::new(),
            },
        ] {
            shell.begin_file_mutation("node", change.clone(), cx);
            let pending = shell.areas.workspace.code.mutation.as_ref().unwrap();
            let (scope, ticket) = (pending.scope.clone(), pending.ticket);
            // The actor selected a connection, but local ids/protocol or
            // pre-handler policy can still prove no filesystem effect.
            shell.file_mutation_answered_for(&scope, &change, ticket, Err(problem), true, cx);
            assert!(shell.areas.workspace.code.mutation.is_none());
            assert!(shell.editing_for(&key).is_some());
            assert_eq!(shell.editing_for(&key).unwrap().model.content(), "saved");
        }
        shell.begin_file_mutation("node", change.clone(), cx);
        let pending = shell.areas.workspace.code.mutation.as_ref().unwrap();
        let (scope, ticket) = (pending.scope.clone(), pending.ticket);
        shell.file_mutation_answered_for(
            &scope,
            &change,
            ticket,
            Err(Problem::Gone("connection retired after polling".into())),
            true,
            cx,
        );
        assert!(
            shell
                .areas
                .workspace
                .code
                .mutation
                .as_ref()
                .unwrap()
                .uncertain
        );
        assert!(shell.editing_for(&key).is_some());
    });
}
