//! The actual Explorer and file-pane render paths share one live editor.
use super::*;
use crate::areas::layout::registry::PanelFocus;
use crate::areas::workspace::code::{editing, files, io_tests};
use gpui::{AppContext as _, TestAppContext};
use vessel_app_kit::ui::{Entity, IntoElement, Render, div};
use vessel_app_view::layout::{PaneId, Placement};

struct Probe {
    shell: Entity<Shell>,
    workspace: LayoutKey,
    tab: TabId,
    file: FileKey,
}
impl Probe {
    fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let mut address = None;
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            shell.areas.workspace.code.open = true;
            shell.areas.workspace.code.tab = super::super::Tab::Files;
            shell.route = crate::routes::Route::Session;
            let root = io_tests::root();
            let (origin, workspace) = tests::opening(&mut shell, root);
            let tab = shell
                .open_workspace_file(
                    &origin,
                    "a.txt".parse().unwrap(),
                    None,
                    Placement::Focused,
                    window,
                    cx,
                )
                .unwrap();
            let file = shell.areas.workspace.code.file_key().unwrap();
            let ticket = shell.editing_for(&file).unwrap().reading.unwrap().0;
            shell.editor_read_answered_for(
                &file,
                ticket,
                &editing::whole_read(root, file.path.clone()),
                Ok(io_tests::text("baseline")),
                cx,
            );
            shell.editor_frame_for(&file, window, cx);
            address = Some((workspace, tab, file));
            shell
        });
        let (workspace, tab, file) = address.unwrap();
        Self {
            shell,
            workspace,
            tab,
            file,
        }
    }
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            let tokens = Tokens::of(vessel_app_kit::Scheme::Dark);
            let file = &self.file;
            let target = Target::File {
                root: file.root,
                path: file.path.clone(),
            };
            let panel = PanelContext {
                workspace: &self.workspace,
                pane: PaneId::MAIN,
                tab: self.tab,
                target: &target,
                focus: PanelFocus::Focused,
            };
            let pane = render_file_panel(shell, &panel, &tokens, window, cx);
            super::super::frame::code_beside_with_explorer(
                shell,
                div().flex_1().h_full().flex().flex_col().child(pane),
                &tokens,
                window,
                cx,
                files::render,
            )
        })
    }
}

#[gpui::test]
fn explorer_keeps_the_tree_while_bound_file_paints_the_existing_editor(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(900.0)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("code-file-tree").is_some(),
        "Explorer must remain the tree"
    );
    assert!(
        cx.debug_bounds("code-back").is_none(),
        "pane uses layout navigation"
    );
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let file = shell.areas.workspace.code.file_key().unwrap();
            shell
                .editing_for(&file)
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .focus(window, cx);
        });
    });
    cx.simulate_input("accepted");
    probe.read_with(cx, |probe, cx| {
        let shell = probe.shell.read(cx);
        let file = &shell.areas.workspace.code.file_tabs.bindings
            [&(probe.workspace.clone(), probe.tab)]
            .file;
        let editing = shell
            .areas
            .workspace
            .code
            .viewer_at(file)
            .unwrap()
            .edit
            .as_ref()
            .unwrap();
        assert!(
            editing
                .editor
                .as_ref()
                .unwrap()
                .text(cx)
                .contains("accepted")
        );
        assert_eq!(
            editing.model.content(),
            editing.editor.as_ref().unwrap().text(cx)
        );
        assert!(editing.model.modified());
    });
}

#[gpui::test]
fn focused_retained_file_routes_find_and_hidden_pane_refuses_it(cx: &mut TestAppContext) {
    use vessel_app_kit::code_editor::FindState;
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(900.0)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let a = shell.areas.workspace.code.file_key().unwrap();
            let origin = shell.file_open_context().unwrap();
            let tab = shell
                .open_workspace_file(
                    &origin,
                    "b.txt".parse().unwrap(),
                    None,
                    Placement::Side,
                    window,
                    cx,
                )
                .unwrap();
            let b = shell.areas.workspace.code.file_key().unwrap();
            assert_ne!(a, b);
            let ticket = shell.editing_for(&b).unwrap().reading.unwrap().0;
            shell.editor_read_answered_for(
                &b,
                ticket,
                &editing::whole_read(b.root, b.path.clone()),
                Ok(io_tests::text("second")),
                cx,
            );
            shell.editor_frame_for(&b, window, cx);
            let editor = shell
                .areas
                .workspace
                .code
                .viewer_at(&a)
                .unwrap()
                .edit
                .as_ref()
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .clone();
            editor.focus(window, cx);
            assert!(shell.file_find_at_focus(window, cx));
            assert!(matches!(editor.find_state(cx), FindState::Open { .. }));
            assert_eq!(
                shell
                    .editing_for(&b)
                    .unwrap()
                    .editor
                    .as_ref()
                    .unwrap()
                    .find_state(cx),
                FindState::Closed
            );
            shell
                .update_layout(
                    &probe.workspace,
                    |layout| {
                        assert_eq!(
                            layout.tab(tab).unwrap().target(),
                            &Target::File {
                                root: b.root,
                                path: b.path.clone(),
                            }
                        );
                        Ok(())
                    },
                    cx,
                )
                .unwrap();
            assert!(shell.run_layout_action(
                vessel_app_view::layout::LayoutAction::ToggleFocus,
                window,
                cx
            ));
            assert!(
                !shell.file_find_at_focus(window, cx),
                "hidden old focus cannot claim central Find"
            );
            assert_eq!(shell.areas.workspace.code.file_key().as_ref(), Some(&b));
        });
    });
}

#[gpui::test]
fn the_legacy_unbound_viewer_records_its_actual_back_control(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(Probe::new);
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(900.0)));
    probe.update_in(cx, |probe, _, cx| {
        probe.shell.update(cx, |shell, cx| {
            let _binding = shell
                .areas
                .workspace
                .code
                .file_tabs
                .bindings
                .remove(&(probe.workspace.clone(), probe.tab));
            cx.notify();
        });
    });
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("code-back").is_some(),
        "the negative pane assertion has a real Explorer control"
    );
    assert!(cx.debug_bounds("code-file-tree").is_none());
}
