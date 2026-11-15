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

struct Probe {
    shell: Entity<Shell>,
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let body: AnyElement = self.shell.update(cx, |shell, cx| {
            let editing = shell
                .areas
                .workspace
                .code
                .files
                .viewer
                .as_ref()
                .unwrap()
                .edit
                .as_ref()
                .unwrap();
            editing_view::body(shell, editing, &Tokens::of(Scheme::Dark), cx)
        });
        div().size_full().child(body)
    }
}

#[gpui::test]
fn keyboard_find_replace_and_escape_reach_the_retained_document(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| Probe {
        shell: cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            let root = io_tests::root();
            io_tests::target(&mut shell, root);
            shell.open_file_at("test.rs".parse().unwrap(), None, cx);
            let key = shell.areas.workspace.code.file_key().unwrap();
            let ticket = shell.editing_for(&key).unwrap().reading.unwrap().0;
            let ask = super::super::editing::whole_read(root, key.path);
            shell.editor_read_answered(
                "node",
                ticket,
                &ask,
                io_tests::text("alpha beta alpha"),
                cx,
            );
            shell.editor_frame(window, cx);
            shell
        }),
    });
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
        let editing = probe
            .shell
            .read(cx)
            .areas
            .workspace
            .code
            .files
            .viewer
            .as_ref()
            .unwrap()
            .edit
            .as_ref()
            .unwrap();
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
        panel.replacement.set_text("gamma", window, cx);
        panel.replace(ReplaceScope::All, window, cx);
    });
    probe.read_with(cx, |probe, cx| {
        let editing = probe
            .shell
            .read(cx)
            .areas
            .workspace
            .code
            .files
            .viewer
            .as_ref()
            .unwrap()
            .edit
            .as_ref()
            .unwrap();
        assert_eq!(
            editing.editor.as_ref().unwrap().text(cx),
            "gamma beta gamma"
        );
        assert_eq!(editing.model.content(), "gamma beta gamma");
        assert!(editing.model.modified());
    });
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let editing = probe
            .shell
            .read(cx)
            .areas
            .workspace
            .code
            .files
            .viewer
            .as_ref()
            .unwrap()
            .edit
            .as_ref()
            .unwrap();
        assert_eq!(
            editing.editor.as_ref().unwrap().find_state(cx),
            FindState::Closed
        );
    });
}
