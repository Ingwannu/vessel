//! Correlation is exercised against the actual retained shell resources,
//! with the link, updater and filesystem absent.

use gpui::TestAppContext;
use vessel_proto::workspace::{FileContent, FileRead, FileVersion, ReadFile};
use vessel_proto::{ProjectId, SessionId};

use super::*;

pub(super) fn root() -> WorkspaceRoot {
    WorkspaceRoot::Project {
        project: ProjectId::random(),
    }
}

pub(super) fn target(shell: &mut Shell, root: WorkspaceRoot) {
    shell.areas.workspace.code.target = Some(Target {
        admission: None,
        machine: "node".to_owned(),
        session: SessionId::random(),
        place: Place::Root(root),
        cwd: Some("/fixture".into()),
    });
}

fn read(root: WorkspaceRoot, path: &str) -> ReadFile {
    ReadFile {
        root,
        path: path.parse().unwrap(),
        offset: 0,
        limit: None,
    }
}

pub(super) fn text(text: &str) -> FileRead {
    FileRead {
        version: FileVersion {
            size: u64::try_from(text.len()).unwrap(),
            modified: 0,
            hash: None,
        },
        content: FileContent::Text {
            text: text.to_owned(),
            next: None,
        },
    }
}

#[gpui::test]
fn readonly_refresh_ignores_late_reply_for_same_path(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = root();
        target(shell, root);
        shell.areas.workspace.code.files.narrow.set(true);
        shell.open_file_at("a.txt".parse().unwrap(), None, cx);
        let old = shell
            .areas
            .workspace
            .code
            .files
            .viewer
            .as_ref()
            .unwrap()
            .reading
            .unwrap();
        shell.reread_files();
        let current = shell
            .areas
            .workspace
            .code
            .files
            .viewer
            .as_ref()
            .unwrap()
            .reading
            .unwrap();
        assert_ne!(old, current);
        shell.file_read_answered("node", &read(root, "a.txt"), current, Ok(text("new")), cx);
        shell.file_read_answered("node", &read(root, "a.txt"), old, Ok(text("old")), cx);
        let viewer = shell.areas.workspace.code.files.viewer.as_ref().unwrap();
        assert!(matches!(&viewer.text, Shown::Text { lines, .. } if lines == &["new"]));
    });
}

#[gpui::test]
fn retained_editor_receives_only_its_original_ticket_and_scope(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = root();
        target(shell, root);
        shell.open_file_at("a.txt".parse().unwrap(), None, cx);
        let a = shell.areas.workspace.code.file_key().unwrap();
        let old = shell.editing_for(&a).unwrap().reading.unwrap().0;
        shell.open_file_at("b.txt".parse().unwrap(), None, cx);
        let b = shell.areas.workspace.code.file_key().unwrap();
        let current = shell.editing_for(&b).unwrap().reading.unwrap().0;
        shell.editor_read_answered("node", old, &read(root, "b.txt"), Ok(text("wrong")), cx);
        assert_eq!(shell.editing_for(&b).unwrap().model.content(), "");
        shell.editor_read_answered(
            "other",
            current,
            &read(root, "b.txt"),
            Ok(text("wrong node")),
            cx,
        );
        assert_eq!(shell.editing_for(&b).unwrap().model.content(), "");
        shell.editor_read_answered(
            "node",
            old,
            &read(root, "a.txt"),
            Ok(text("retained a")),
            cx,
        );
        assert_eq!(shell.editing_for(&a).unwrap().model.content(), "retained a");
        shell.editor_read_answered(
            "node",
            current,
            &read(root, "b.txt"),
            Ok(text("active b")),
            cx,
        );
        assert_eq!(shell.editing_for(&b).unwrap().model.content(), "active b");
        shell.open_file_at("a.txt".parse().unwrap(), None, cx);
        assert_eq!(shell.areas.workspace.code.file_key(), Some(a.clone()));
        assert_eq!(shell.editing_for(&a).unwrap().model.content(), "retained a");
    });
}

#[gpui::test]
fn reload_decisions_keep_the_original_file_and_dialog_opening(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = root();
        target(shell, root);
        shell.open_file_at("a.txt".parse().unwrap(), None, cx);
        let a = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell.editing_for(&a).unwrap().reading.unwrap().0;
        shell.editor_read_answered("node", ticket, &read(root, "a.txt"), Ok(text("a")), cx);
        let old = shell.next_file_ticket().unwrap();
        let current = shell.next_file_ticket().unwrap();
        shell.editing_for(&a).unwrap().reload = Some(editing::ReloadPrompt {
            ticket: current,
            dialog: vessel_app_kit::DialogState::new(cx),
        });
        shell.open_file_at("b.txt".parse().unwrap(), None, cx);
        let b = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell.editing_for(&b).unwrap().reading.unwrap().0;
        shell.editor_read_answered("node", ticket, &read(root, "b.txt"), Ok(text("b")), cx);
        let _step = shell.editing_for(&b).unwrap().model.edit("unsaved b");
        shell.editor_reload_answered(&a, old, editing::ReloadDecision::Reload, cx);
        assert_eq!(
            shell
                .editing_for(&a)
                .unwrap()
                .reload
                .as_ref()
                .unwrap()
                .ticket,
            current
        );
        shell.editor_reload_answered(&a, current, editing::ReloadDecision::Keep, cx);
        assert!(shell.editing_for(&a).unwrap().reload.is_none());
        assert_eq!(shell.editing_for(&b).unwrap().model.content(), "unsaved b");
        assert!(shell.editing_for(&b).unwrap().model.modified());
        assert_eq!(shell.areas.workspace.code.file_key(), Some(b));
    });
}

#[gpui::test]
fn uncertain_original_write_is_retained_after_local_undo(cx: &mut TestAppContext) {
    use vessel_app_view::workspace::editor::{Step, WriteState};
    use vessel_app_view::workspace::save::Saved;
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = root();
        target(shell, root);
        shell.open_file_at("original.txt".parse().unwrap(), None, cx);
        let key = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell.editing_for(&key).unwrap().reading.unwrap().0;
        shell.editor_read_answered(
            "node",
            ticket,
            &read(root, "original.txt"),
            Ok(text("a")),
            cx,
        );
        let writing = shell.next_file_ticket().unwrap();
        let editing = shell.editing_for(&key).unwrap();
        let _step = editing.model.edit("b");
        let Step::Write { sequence, .. } = editing.model.save() else {
            panic!("write")
        };
        editing.writing = Some((writing, sequence));
        shell.editor_saved(
            "node",
            writing,
            &root,
            &key.path,
            Saved::Uncertain(Problem::Gone("lost reply".into())),
            cx,
        );
        let editing = shell.editing_for(&key).unwrap();
        assert_eq!(editing.writing, Some((writing, sequence)));
        let _step = editing.model.edit("a");
        assert!(!editing.model.modified());
        assert_eq!(editing.model.write_state(), Some(WriteState::Uncertain));
        assert_eq!(editing.model.save(), Step::None);
        assert_eq!(editing.model.reload(), Step::None);
        shell.open_file_at("next.txt".parse().unwrap(), None, cx);
        assert!(shell.areas.workspace.code.files.retained.contains_key(&key));
        assert_eq!(
            shell.editing_for(&key).unwrap().writing,
            Some((writing, sequence))
        );
    });
}
