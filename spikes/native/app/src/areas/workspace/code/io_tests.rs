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
        admission: Some(super::scope::Acquisition {
            context: vessel_app_view::connection::ConnectionEpochs::default()
                .admit(
                    vessel_app_view::connection::MachineKey::from_book("node"),
                    "00000000-0000-4000-8000-000000000001".parse().unwrap(),
                )
                .unwrap(),
            ticket: shell.next_file_ticket().unwrap(),
        }),
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
        assert!(matches!(&viewer.text, Shown::Text { source, .. } if source.line(0) == Some("new") && source.line_count() == 1));
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
        let scope = shell
            .areas
            .workspace
            .code
            .viewer_for(&key)
            .unwrap()
            .scope
            .clone();
        let editing = shell.editing_for(&key).unwrap();
        let _step = editing.model.edit("b");
        let Step::Write { sequence, .. } = editing.model.save() else {
            panic!("write")
        };
        editing.writing = Some(editing::WriteTicket {
            ticket: writing,
            sequence,
            scope,
        });
        shell.editor_saved(
            "node",
            writing,
            &root,
            &key.path,
            Saved::Uncertain(Problem::Gone("lost reply".into())),
            cx,
        );
        let editing = shell.editing_for(&key).unwrap();
        assert_eq!(
            editing
                .writing
                .as_ref()
                .map(|attempt| (attempt.ticket, attempt.sequence)),
            Some((writing, sequence))
        );
        let _step = editing.model.edit("a");
        assert!(!editing.model.modified());
        assert_eq!(editing.model.write_state(), Some(WriteState::Uncertain));
        assert_eq!(editing.model.save(), Step::None);
        assert_eq!(editing.model.reload(), Step::None);
        shell.open_file_at("next.txt".parse().unwrap(), None, cx);
        assert!(shell.areas.workspace.code.files.retained.contains_key(&key));
        assert_eq!(
            shell
                .editing_for(&key)
                .unwrap()
                .writing
                .as_ref()
                .map(|attempt| (attempt.ticket, attempt.sequence)),
            Some((writing, sequence))
        );
    });
}

// Compatibility helpers are confined to isolated tests. Product dispatch receives
// original FileScope+ticket and never reconstructs an identity from an alias.
impl Shell {
    fn fixture_key(
        &self,
        machine: &str,
        root: WorkspaceRoot,
        path: &vessel_proto::workspace::RelPath,
    ) -> Option<buffers::FileKey> {
        let files = &self.areas.workspace.code.files;
        files
            .viewer
            .iter()
            .chain(files.retained.values())
            .find(|viewer| {
                viewer.scope.connection().machine().as_str() == machine
                    && viewer.scope.root() == root
                    && viewer.path == *path
            })
            .map(|viewer| buffers::FileKey::new(&viewer.scope, path.clone()))
    }
    pub(super) fn file_read_answered(
        &mut self,
        machine: &str,
        ask: &ReadFile,
        ticket: vessel_app_view::workspace::file_ticket::FileTicket,
        result: Result<FileRead, Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(key) = self.fixture_key(machine, ask.root, &ask.path) {
            self.file_read_answered_for(&key, ask, ticket, result, cx);
        }
    }
    pub(super) fn editor_read_answered(
        &mut self,
        machine: &str,
        ticket: vessel_app_view::workspace::file_ticket::FileTicket,
        ask: &ReadFile,
        result: Result<FileRead, Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(key) = self.fixture_key(machine, ask.root, &ask.path) {
            self.editor_read_answered_for(&key, ticket, ask, result, cx);
        }
    }
    pub(super) fn editor_saved(
        &mut self,
        machine: &str,
        ticket: vessel_app_view::workspace::file_ticket::FileTicket,
        root: &WorkspaceRoot,
        path: &vessel_proto::workspace::RelPath,
        saved: vessel_app_view::workspace::save::Saved,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(key) = self.fixture_key(machine, *root, path) {
            let scope = self
                .areas
                .workspace
                .code
                .viewer_for(&key)
                .unwrap()
                .scope
                .clone();
            self.editor_saved_for(&key, &scope, ticket, true, saved, cx);
        }
    }
    pub(super) fn begin_file_mutation(
        &mut self,
        machine: &str,
        change: vessel_app_link::workspace::PathChange,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(scope) = self
            .areas
            .workspace
            .code
            .scope()
            .filter(|scope| scope.connection().machine().as_str() == machine)
        {
            self.begin_file_mutation_for(scope, change, cx);
        }
    }
    pub(super) fn file_mutation_answered(
        &mut self,
        machine: &str,
        change: &vessel_app_link::workspace::PathChange,
        ticket: vessel_app_view::workspace::file_ticket::FileTicket,
        result: Result<(), Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        let scope = self
            .areas
            .workspace
            .code
            .mutation
            .as_ref()
            .filter(|pending| pending.scope.connection().machine().as_str() == machine)
            .map(|pending| pending.scope.clone());
        if let Some(scope) = scope {
            self.file_mutation_answered_for(&scope, change, ticket, result, true, cx);
        }
    }
}

#[gpui::test]
fn old_same_scope_folder_refresh_cannot_replace_the_latest_listing(cx: &mut TestAppContext) {
    use vessel_proto::workspace::{DirEntry, DirListing, EntryKind, ListDir, RelPath};
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, _| {
        let root = root();
        target(shell, root);
        let dir = RelPath::root();
        shell.read_folder(&dir);
        let old = shell.areas.workspace.code.files.listing_tickets[&dir];
        shell.reread_files();
        let current = shell.areas.workspace.code.files.listing_tickets[&dir];
        assert_ne!(old, current);
        let ask = ListDir { root, dir: dir.clone(), ignored: true };
        let listed = |name: &str| DirListing { entries: vec![DirEntry { name: name.into(), kind: EntryKind::File, size: None, ignored: false }], more: false };
        shell.folder_read("node", &ask, current, Ok(listed("current")));
        shell.folder_read("node", &ask, old, Ok(listed("stale")));
        assert!(matches!(&shell.areas.workspace.code.files.listings[&dir], vessel_app_view::workspace::file_tree::Listing::Listed(listing) if listing.entries[0].name == "current"));
    });
}

#[gpui::test]
fn deferred_editor_creation_stays_with_its_original_retained_owner(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let root = root();
        target(shell, root);
        shell.open_file_at("a.rs".parse().unwrap(), None, cx);
        let a = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell.editing_for(&a).unwrap().reading.unwrap().0;
        shell.editor_read_answered("node", ticket, &read(root, "a.rs"), Ok(text("a")), cx);
        shell.open_file_at("b.rs".parse().unwrap(), None, cx);
        let b = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell.editing_for(&b).unwrap().reading.unwrap().0;
        shell.editor_read_answered("node", ticket, &read(root, "b.rs"), Ok(text("b")), cx);
        shell.editor_frame_for(&a, window, cx);
        assert!(shell.editing_for(&a).unwrap().editor.is_some());
        assert!(shell.editing_for(&b).unwrap().editor.is_none());
        assert_eq!(shell.areas.workspace.code.file_key(), Some(b.clone()));
        shell.leave_file_for(&a, cx);
        assert_eq!(shell.areas.workspace.code.file_key(), Some(b));
    });
}

#[gpui::test]
fn initial_large_editor_source_reacquires_whole_readonly_instead_of_installing_a_prefix(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = root();
        target(shell, root);
        shell.open_file_at("large.rs".parse().unwrap(), None, cx);
        let key = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell.editing_for(&key).unwrap().reading.unwrap().0;
        let mut prefix = text("first page");
        prefix.version.size = 2 * 1024 * 1024;
        prefix.content = FileContent::Text { text: "first page".into(), next: Some(10) };
        let ask = read(root, "large.rs");
        shell.editor_read_answered_for(&key, ticket, &ask, Ok(prefix), cx);
        let viewer = shell.areas.workspace.code.viewer_at(&key).unwrap();
        assert!(viewer.edit.is_none());
        assert!(matches!(viewer.text, Shown::Loading));
        let whole_ticket = viewer.reading.unwrap();
        let source = "x".repeat(2 * 1024 * 1024);
        shell.file_read_answered_for(&key, &ask, whole_ticket, Ok(text(&source)), cx);
        assert!(matches!(&shell.areas.workspace.code.viewer_at(&key).unwrap().text,
            Shown::Text { source: whole, states, more: false, .. } if whole.as_ref() == source && states.is_empty()));
    });
}

#[gpui::test]
fn oversize_source_has_an_explicit_fallback_without_a_second_acquisition(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = root();
        target(shell, root);
        shell.open_file_at("huge.txt".parse().unwrap(), None, cx);
        let key = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell.editing_for(&key).unwrap().reading.unwrap().0;
        let mut oversized = text("x");
        oversized.version.size =
            vessel_app_view::workspace::source::MAX_SOURCE_BYTES.saturating_add(1);
        shell.editor_read_answered_for(&key, ticket, &read(root, "huge.txt"), Ok(oversized), cx);
        let viewer = shell.areas.workspace.code.viewer_at(&key).unwrap();
        assert!(matches!(viewer.text, Shown::TooLarge));
        assert!(viewer.edit.is_none());
        assert!(viewer.reading.is_none());
    });
}

#[gpui::test]
fn readonly_payload_reservation_evicts_only_disposable_parked_sources(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = root();
        target(shell, root);
        shell.areas.workspace.code.files.narrow.set(true);
        shell.open_file_at("a.txt".parse().unwrap(), None, cx);
        let a = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell
            .areas
            .workspace
            .code
            .viewer_at(&a)
            .unwrap()
            .reading
            .unwrap();
        shell.file_read_answered_for(&a, &read(root, "a.txt"), ticket, Ok(text("a")), cx);
        shell.open_file_at("b.txt".parse().unwrap(), None, cx);
        let b = shell.areas.workspace.code.file_key().unwrap();
        // Exercise the reservation without allocating a budget-sized string.
        assert!(shell.reserve_readonly_source(&b, super::buffers::MAX_READONLY_SOURCE));
        assert!(!shell.areas.workspace.code.files.retained.contains_key(&a));
        let ticket = shell
            .areas
            .workspace
            .code
            .viewer_at(&b)
            .unwrap()
            .reading
            .unwrap();
        shell.file_read_answered_for(&b, &read(root, "b.txt"), ticket, Ok(text("visible")), cx);
        // A late retained acquisition cannot evict the currently visible source.
        assert!(!shell.reserve_readonly_source(&a, super::buffers::MAX_READONLY_SOURCE));
        assert!(shell.areas.workspace.code.viewer_at(&b).is_some());
    });
}
