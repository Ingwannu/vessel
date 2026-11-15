//! Correlation is exercised against the actual retained shell resources,
//! with the link, updater and filesystem absent.

use gpui::TestAppContext;
use vessel_proto::workspace::{FileContent, FileRead, FileVersion, ReadFile};
use vessel_proto::{ProjectId, SessionId};

use super::*;

fn root() -> WorkspaceRoot {
    WorkspaceRoot::Project {
        project: ProjectId::new(),
    }
}

fn target(shell: &mut Shell, root: WorkspaceRoot) {
    shell.areas.workspace.code.target = Some(Target {
        machine: "node".to_owned(),
        session: SessionId::new(),
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

fn text(text: &str) -> Result<FileRead, Problem> {
    Ok(FileRead {
        version: FileVersion {
            size: u64::try_from(text.len()).unwrap(),
            modified: 0,
            hash: None,
        },
        content: FileContent::Text {
            text: text.to_owned(),
            next: None,
        },
    })
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
        shell.file_read_answered("node", &read(root, "a.txt"), current, text("new"), cx);
        shell.file_read_answered("node", &read(root, "a.txt"), old, text("old"), cx);
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
        shell.editor_read_answered("node", old, &read(root, "b.txt"), text("wrong"), cx);
        assert_eq!(shell.editing_for(&b).unwrap().model.content(), "");
        shell.editor_read_answered(
            "other",
            current,
            &read(root, "b.txt"),
            text("wrong node"),
            cx,
        );
        assert_eq!(shell.editing_for(&b).unwrap().model.content(), "");
        shell.editor_read_answered("node", old, &read(root, "a.txt"), text("retained a"), cx);
        assert_eq!(shell.editing_for(&a).unwrap().model.content(), "retained a");
        shell.editor_read_answered("node", current, &read(root, "b.txt"), text("active b"), cx);
        assert_eq!(shell.editing_for(&b).unwrap().model.content(), "active b");
        shell.open_file_at("a.txt".parse().unwrap(), None, cx);
        assert_eq!(shell.areas.workspace.code.file_key(), Some(a.clone()));
        assert_eq!(shell.editing_for(&a).unwrap().model.content(), "retained a");
    });
}
