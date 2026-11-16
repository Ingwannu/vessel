//! The real editor/search panel updates the guarded source model after
//! replacement; an isolated shell prevents any node or filesystem I/O.

use gpui::{AppContext as _, TestAppContext};
use vessel_app_kit::code_editor::{FindState, MatchCount};
use vessel_app_kit::ui::{
    AnyElement, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _, Window, div,
};
use vessel_app_kit::{Scheme, Tokens};

use super::super::{editing_view, io_tests};
use super::*;

fn editing(shell: &Shell) -> &super::super::editing::Editing {
    shell
        .areas
        .workspace
        .code
        .files
        .viewer
        .as_ref()
        .unwrap()
        .edit
        .as_ref()
        .unwrap()
}

struct Probe {
    shell: Entity<Shell>,
    file: FileKey,
}

impl Probe {
    fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let mut file = None;
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            let root = io_tests::root();
            io_tests::target(&mut shell, root);
            shell.open_file_at("test.rs".parse().unwrap(), None, cx);
            let key = shell.areas.workspace.code.file_key().unwrap();
            let ticket = shell.editing_for(&key).unwrap().reading.unwrap().0;
            let ask = super::super::editing::whole_read(root, key.path.clone());
            shell.editor_read_answered(
                "node",
                ticket,
                &ask,
                Ok(io_tests::text("alpha beta alpha")),
                cx,
            );
            shell.editor_frame(window, cx);
            file = Some(key);
            shell
        });
        Self {
            shell,
            file: file.unwrap(),
        }
    }
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let body: AnyElement = self.shell.update(cx, |shell, cx| {
            let editing = shell
                .areas
                .workspace
                .code
                .viewer_at(&self.file)
                .unwrap()
                .edit
                .as_ref()
                .unwrap();
            editing_view::body(shell, editing, &self.file, &Tokens::of(Scheme::Dark), cx)
        });
        div().size_full().child(body)
    }
}

#[gpui::test]
fn keyboard_find_replace_and_escape_reach_the_retained_document(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    cx.update(|window, _| window.activate_window());
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell
                .areas
                .workspace
                .code
                .files
                .viewer
                .as_ref()
                .unwrap()
                .edit
                .as_ref()
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .focus(window, cx);
        });
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("ctrl-f");
    cx.run_until_parked();
    cx.simulate_input("alpha");
    cx.run_until_parked();
    let panel = probe.read_with(cx, |probe, cx| {
        let editing = editing(probe.shell.read(cx));
        assert!(matches!(
            editing.editor.as_ref().unwrap().find_state(cx),
            FindState::Open {
                total: MatchCount(2),
                ..
            }
        ));
        editing.find.clone().unwrap()
    });
    panel.update_in(cx, |panel, window, cx| {
        panel
            .editor
            .set_editability(vessel_app_kit::Editability::ReadOnly, cx);
        panel.replacement.set_text("blocked", window, cx);
        panel.replace(ReplaceScope::All, window, cx);
        assert_eq!(panel.editor.text(cx), "alpha beta alpha");
        panel
            .editor
            .set_editability(vessel_app_kit::Editability::Editable, cx);
        panel.replacement.set_text("gamma", window, cx);
        panel.replace(ReplaceScope::All, window, cx);
    });
    probe.read_with(cx, |probe, cx| {
        let editing = editing(probe.shell.read(cx));
        assert_eq!(
            editing.editor.as_ref().unwrap().text(cx),
            "gamma beta gamma"
        );
        assert_eq!(editing.model.content(), "gamma beta gamma");
        assert!(editing.model.modified());
    });
    cx.simulate_keystrokes("ctrl-s");
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let editing = editing(probe.shell.read(cx));
        assert!(
            editing.writing.is_some(),
            "manual save must use the guarded request path"
        );
    });
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let editing = editing(probe.shell.read(cx));
        assert_eq!(
            editing.editor.as_ref().unwrap().find_state(cx),
            FindState::Closed
        );
    });
}

#[gpui::test]
fn source_navigation_uses_scalars_and_clamps_stale_line_targets(cx: &mut TestAppContext) {
    use std::num::NonZeroU32;
    use vessel_app_view::workspace::file_location::FileLocation;
    use vessel_proto::lsp::{Position, Range};
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    cx.update(|window, _| window.activate_window());
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let editor = editing(shell).editor.as_ref().unwrap().clone();
            editor.set_text("한😀글\nsecond", window, cx);
            let location = FileLocation::try_from(Range {
                start: Position {
                    line: NonZeroU32::MIN,
                    column: NonZeroU32::new(2).unwrap(),
                },
                end: Position {
                    line: NonZeroU32::MIN,
                    column: NonZeroU32::new(3).unwrap(),
                },
            })
            .unwrap();
            super::super::editor_io::reveal_location(&editor, location, window, cx);
            assert_eq!(editor.cursor(cx), (1, 3));
            assert_eq!(editor.selected_lines(cx), Some((1, 1)));
        });
    });
    cx.run_until_parked();
    cx.simulate_input("X");
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let editor = editing(shell).editor.as_ref().unwrap().clone();
            assert_eq!(editor.text(cx), "한X글\nsecond");
            assert_eq!(editing(shell).model.content(), "한X글\nsecond");
            super::super::editor_io::reveal_location(
                &editor,
                FileLocation::line(NonZeroU32::MAX),
                window,
                cx,
            );
            assert_eq!(editor.cursor(cx), (2, 1));
        });
    });
}

#[gpui::test]
fn global_find_yields_to_actual_editor_focus_but_not_its_displayed_path(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.route = crate::routes::Route::Session;
            shell.areas.workspace.code.open = true;
            shell.areas.workspace.code.tab = super::super::Tab::Files;
            let editor = editing(shell).editor.as_ref().unwrap().clone();
            editor.focus(window, cx);
            assert!(shell.file_find_at_focus(window, cx));
            assert!(matches!(editor.find_state(cx), FindState::Open { .. }));
            shell.areas.session.composer.focus(window, cx);
            assert!(
                !shell.file_find_at_focus(window, cx),
                "a displayed editor cannot steal the composer's Find"
            );
            shell.areas.workspace.code.open = false;
            editor.focus(window, cx);
            assert!(
                !shell.file_find_at_focus(window, cx),
                "an unmounted legacy owner does not route keys"
            );
        });
    });
}

// Silent engine replacement supplies a deterministic accepted-edit / queued-
// mirror barrier. This tests the real engine, not OS IME delivery ordering.
#[gpui::test]
fn destructive_admission_observes_accepted_text_before_its_mirror(cx: &mut TestAppContext) {
    use vessel_app_kit::code_editor::FindCase;
    use vessel_app_link::workspace::PathChange;
    use vessel_proto::workspace::DeletePath;

    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let key = shell.areas.workspace.code.file_key().unwrap();
            let scope = shell.areas.workspace.code.scope().unwrap();
            let editor = editing(shell).editor.as_ref().unwrap().clone();
            editor.open_find(cx);
            editor.find_query("alpha", FindCase::Sensitive, cx);
            assert_eq!(
                editor.find_replace("updated", ReplaceScope::All, window, cx),
                MatchCount(2)
            );
            assert_eq!(editor.text(cx), "updated beta updated");
            assert_eq!(editing(shell).model.content(), "alpha beta alpha");
            assert!(!editing(shell).model.modified());

            shell.begin_file_mutation_for(
                scope,
                PathChange::Delete(DeletePath {
                    root: key.root,
                    path: key.path.clone(),
                    recursive: false,
                }),
                cx,
            );
            assert!(shell.areas.workspace.code.mutation.is_none());
            assert!(shell.areas.workspace.code.files.actions.refused.is_some());
            assert_eq!(editing(shell).model.content(), "updated beta updated");
            assert!(editing(shell).model.modified());
            assert!(editing(shell).writing.is_none());
            shell.editor_changed_for(&key, window, cx);
            assert_eq!(editing(shell).model.content(), editor.text(cx));
            assert!(editing(shell).model.modified());
            assert!(editing(shell).writing.is_none());
        });
    });
}

#[gpui::test]
fn accepted_text_before_pending_projection_survives_delete_and_late_frame(cx: &mut TestAppContext) {
    use vessel_app_kit::code_editor::FindCase;
    use vessel_app_link::workspace::PathChange;
    use vessel_app_view::workspace::editor::{ConflictKind, Step};
    use vessel_proto::workspace::DeletePath;

    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let key = shell.areas.workspace.code.file_key().unwrap();
            let scope = shell.areas.workspace.code.scope().unwrap();
            let editor = editing(shell).editor.as_ref().unwrap().clone();
            let step = shell.editing_for(&key).unwrap().model.reconcile();
            assert!(matches!(step, Step::Read { .. }));
            shell.send_step_for(&key, step);
            let ticket = shell.editing_for(&key).unwrap().reading.unwrap().0;
            shell.editor_read_answered_for(
                &key,
                ticket,
                &super::super::editing::whole_read(key.root, key.path.clone()),
                Ok(io_tests::text("newer disk")),
                cx,
            );
            assert_eq!(editing(shell).model.content(), "newer disk");
            assert_eq!(editor.text(cx), "alpha beta alpha");
            assert!(editing(shell).stale);
            shell.mirror_editor_for(&key, cx);
            assert_eq!(
                editing(shell).model.content(),
                "newer disk",
                "unchanged old display cannot undo the new model"
            );
            editor.open_find(cx);
            editor.find_query("alpha", FindCase::Sensitive, cx);
            assert_eq!(
                editor.find_replace("accepted", ReplaceScope::All, window, cx),
                MatchCount(2)
            );
            // Its owner notification is still withheld at destructive admission.
            shell.begin_file_mutation_for(
                scope,
                PathChange::Delete(DeletePath {
                    root: key.root,
                    path: key.path.clone(),
                    recursive: false,
                }),
                cx,
            );
            assert!(shell.areas.workspace.code.mutation.is_none());
            assert_eq!(editing(shell).model.content(), "accepted beta accepted");
            assert_eq!(
                editing(shell).model.conflict(),
                Some((ConflictKind::Changed, true))
            );
            assert!(!editing(shell).stale);
            assert!(editing(shell).writing.is_none());
            shell.editor_changed_for(&key, window, cx);
            shell.editor_frame_for(&key, window, cx);
            assert_eq!(editor.text(cx), "accepted beta accepted");
            assert_eq!(editing(shell).model.content(), editor.text(cx));
            assert_eq!(shell.editing_for(&key).unwrap().model.save(), Step::None);
        });
    });
}

#[gpui::test]
fn actual_document_guard_observes_unmirrored_engine_text_and_retains_it(cx: &mut TestAppContext) {
    use vessel_app_kit::code_editor::FindCase;
    use vessel_app_view::desktop::quit::DocumentReadiness;
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let key = shell.areas.workspace.code.file_key().unwrap();
            let editor = editing(shell).editor.as_ref().unwrap().clone();
            assert_eq!(
                super::super::lifecycle::check_documents(shell, cx),
                DocumentReadiness::Ready
            );
            assert!(
                !shell
                    .areas
                    .workspace
                    .code
                    .viewer_at(&key)
                    .unwrap()
                    .can_evict(),
                "a context-free cache cannot certify an instantiated editor"
            );
            editor.open_find(cx);
            editor.find_query("alpha", FindCase::Sensitive, cx);
            assert_eq!(
                editor.find_replace("accepted", ReplaceScope::All, window, cx),
                MatchCount(2)
            );
            assert!(!editing(shell).model.modified());
            let before = editing(shell).model.content().to_owned();
            assert_eq!(
                super::super::lifecycle::check_documents(shell, cx),
                DocumentReadiness::Dirty
            );
            assert_eq!(
                shell.file_documents_for_node(key.node, cx),
                DocumentReadiness::Dirty
            );
            assert_eq!(
                editing(shell).model.content(),
                before,
                "readiness never mutates the model"
            );
            assert!(editing(shell).writing.is_none());
            assert!(shell.park_file());
            assert_eq!(
                super::super::lifecycle::check_documents(shell, cx),
                DocumentReadiness::Dirty
            );
            assert!(shell.areas.workspace.code.files.retained.contains_key(&key));
            assert_eq!(editor.text(cx), "accepted beta accepted");
        });
    });
}

#[gpui::test]
fn equal_text_after_accepted_edits_survives_a_pending_disk_projection(cx: &mut TestAppContext) {
    use vessel_app_kit::code_editor::FindCase;
    use vessel_app_view::desktop::quit::DocumentReadiness;
    use vessel_app_view::workspace::editor::{ConflictKind, Step};
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let key = shell.areas.workspace.code.file_key().unwrap();
            let editor = editing(shell).editor.as_ref().unwrap().clone();
            let before = editor.revision(cx);
            let step = shell.editing_for(&key).unwrap().model.reconcile();
            shell.send_step_for(&key, step);
            let ticket = shell.editing_for(&key).unwrap().reading.unwrap().0;
            shell.editor_read_answered_for(
                &key,
                ticket,
                &super::super::editing::whole_read(key.root, key.path.clone()),
                Ok(io_tests::text("newer disk")),
                cx,
            );
            assert!(editing(shell).stale);
            assert_eq!(editing(shell).model.content(), "newer disk");
            assert_eq!(
                super::super::lifecycle::check_documents(shell, cx),
                DocumentReadiness::Ready,
                "an unchanged old projection is not an edit"
            );
            editor.open_find(cx);
            editor.find_query("alpha", FindCase::Sensitive, cx);
            assert_eq!(
                editor.find_replace("temporary", ReplaceScope::All, window, cx),
                MatchCount(2)
            );
            editor.find_query("temporary", FindCase::Sensitive, cx);
            assert_eq!(
                editor.find_replace("alpha", ReplaceScope::All, window, cx),
                MatchCount(2)
            );
            assert_eq!(editor.text(cx), "alpha beta alpha");
            assert_ne!(editor.revision(cx), before);
            assert_eq!(editing(shell).model.content(), "newer disk");
            assert_eq!(
                shell.file_documents_for_node(key.node, cx),
                DocumentReadiness::Dirty,
                "equal text with new engine edits cannot admit quit"
            );
            shell.mirror_editor_for(&key, cx);
            assert_eq!(editing(shell).model.content(), "alpha beta alpha");
            assert_eq!(
                editing(shell).model.conflict(),
                Some((ConflictKind::Changed, true))
            );
            shell.editor_changed_for(&key, window, cx);
            shell.editor_frame_for(&key, window, cx);
            assert_eq!(editor.text(cx), "alpha beta alpha");
            assert_eq!(editing(shell).model.content(), editor.text(cx));
            assert!(editing(shell).writing.is_none());
            assert_eq!(shell.editing_for(&key).unwrap().model.save(), Step::None);
        });
    });
}
