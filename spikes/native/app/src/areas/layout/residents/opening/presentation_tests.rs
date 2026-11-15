//! Admission cannot give keyboard ownership beneath a different visible panel.
use super::*;
use crate::areas::layout::residents::opening::tests::{Registry, build, connection};
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_view::connection::ConnectionEpochs;
use vessel_app_view::layout::{Modified, PaneId, Position};
use vessel_proto::workspace::{RelPath, WorkspaceRoot};
use vessel_proto::{NodeId, ProjectId, WorkspaceId};

#[gpui::test]
fn retained_dirty_sheet_refuses_root_input_admission_without_hiding_it(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let mut epochs = ConnectionEpochs::default();
        let registry = Registry(WorkspaceId::random());
        let opening = SessionOpening::resolve(
            &registry,
            connection(&mut epochs, NodeId::random()),
            SessionId::random(),
        )
        .unwrap();
        let mut layout = Layout::default();
        let file = layout
            .open(
                Target::File {
                    root: WorkspaceRoot::Project {
                        project: ProjectId::random(),
                    },
                    path: "unsaved.rs".parse::<RelPath>().unwrap(),
                },
                OpenIntent::Reveal,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::Compact,
            )
            .unwrap();
        layout.set_modified(file, Modified::Dirty).unwrap();
        let before = layout.clone();
        super::super::super::initialized(shell);
        let _old = shell
            .areas
            .layout
            .book
            .borrow_mut()
            .as_mut()
            .unwrap()
            .insert(opening.workspace.clone(), layout);
        let result = shell.admit_session_open(
            &opening,
            &registry,
            |_, _, _| panic!("covered input must not allocate"),
            |_, _| {},
            window,
            cx,
        );
        assert!(matches!(
            result,
            Err(SessionOpeningProblem::Resident(ResidentProblem::Held))
        ));
        assert_eq!(
            shell
                .areas
                .layout
                .book
                .borrow()
                .as_ref()
                .unwrap()
                .get(&opening.workspace),
            Some(&before)
        );
        assert!(shell.areas.session_residents.entries.is_empty());
        assert!(shell.areas.session_residents.focused().is_none());
    });
}

#[gpui::test]
fn successful_reveal_exits_another_panes_maximized_projection(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let mut epochs = ConnectionEpochs::default();
        let registry = Registry(WorkspaceId::random());
        let opening = SessionOpening::resolve(
            &registry,
            connection(&mut epochs, NodeId::random()),
            SessionId::random(),
        )
        .unwrap();
        let mut layout = Layout::default();
        let _agent = layout
            .open(
                Target::Agent(opening.address.session),
                OpenIntent::Reveal,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )
            .unwrap();
        let empty = layout
            .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
            .unwrap();
        super::super::super::initialized(shell);
        let _old = shell
            .areas
            .layout
            .book
            .borrow_mut()
            .as_mut()
            .unwrap()
            .insert(opening.workspace.clone(), layout);
        shell.areas.layout.maximized = Some((opening.workspace.clone(), empty));
        let admitted = shell
            .admit_session_open(&opening, &registry, build, |_, _| {}, window, cx)
            .unwrap();
        assert_eq!(admitted.context.pane, PaneId::MAIN);
        assert!(shell.areas.layout.maximized.is_none());
        let book = shell.areas.layout.book.borrow();
        let layout = book.as_ref().unwrap().get(&opening.workspace).unwrap();
        assert!(
            super::super::super::queries::projected_pane(shell, layout, &opening.workspace, window)
                .is_none_or(|pane| pane == admitted.context.pane)
        );
    });
}
