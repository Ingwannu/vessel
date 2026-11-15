//! Isolated canonical workspace fixtures; compiled UI, no live registry or RPC.
use super::*;
use gpui::TestAppContext;
use vessel_app_link::Link;
use vessel_app_view::{
    connection::{ConnectionEpochs, MachineKey},
    labels::{Catalog, Color, Host},
    layout::LayoutKey,
    machine_book,
};
use vessel_proto::{NodeId, WorkspaceId};

const COMMAND_CAPACITY: usize = 2;

fn install(shell: &mut Shell) -> WorkspaceTarget {
    let connection = ConnectionEpochs::default()
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let key = LayoutKey::Workspace {
        node: connection.node(),
        workspace: WorkspaceId::random(),
    };
    shell.areas.labels.hosts = vec![Host {
        key: connection.node(),
        route: machine_book::Name::parse("fixture").unwrap(),
        connection: Some(connection.clone()),
        state: HostState::Online,
        catalog: Catalog::snapshot([label()]),
    }];
    WorkspaceTarget::new(connection, &key).unwrap()
}
fn label() -> Label {
    Label {
        name: "Ready".parse().unwrap(),
        color: Color::Violet,
    }
}

#[gpui::test]
fn labels_workspace_picker_stays_open_and_waits_for_registry_assignment_projection(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        let target = install(shell);
        shell.labels_picker_open(
            target.clone(),
            vec![label().name],
            EditAccess::Editable,
            window,
            cx,
        );
        let opening = shell.areas.labels.picker.as_ref().unwrap().opening;
        assert!(
            super::overlay(
                shell,
                &vessel_app_kit::Tokens::of(vessel_app_kit::Scheme::Dark),
                window,
                cx
            )
            .is_some()
        );
        shell.labels_picker_toggle(opening, &label(), Assignment::Unassigned, cx);
        let vessel_app_link::Command::Labels(vessel_app_link::labels::Request {
            context,
            ticket,
            ask:
                Command::Assign {
                    workspace,
                    assignment,
                    ..
                },
        }) = commands.try_recv().unwrap()
        else {
            panic!("one explicit assignment");
        };
        assert_eq!(context, *target.connection());
        assert_eq!(workspace, target.workspace());
        assert_eq!(assignment, Assignment::Unassigned);
        shell.labels_picker_toggle(opening, &label(), Assignment::Assigned, cx);
        assert!(commands.try_recv().is_err());
        shell.labels_answer(
            Event {
                context,
                ticket,
                answer: Answer::Assigned {
                    workspace,
                    name: label().name,
                    result: Ok(()),
                },
            },
            cx,
        );
        let picker = shell.areas.labels.picker.as_ref().unwrap();
        assert!(matches!(picker.page, PickerPage::Listing));
        assert_eq!(picker.assigned, vec![label().name]);
        assert!(shell.areas.labels.picker_combo.is_open());
        shell.labels_picker_sync(&target, Vec::new(), EditAccess::ReadOnly, cx);
        let picker = shell.areas.labels.picker.as_ref().unwrap();
        assert!(picker.assigned.is_empty());
        assert_eq!(
            availability(&shell.areas.labels, picker),
            HostState::Offline
        );
        shell.labels_picker_toggle(opening, &label(), Assignment::Assigned, cx);
        assert!(commands.try_recv().is_err());
    });
}

#[gpui::test]
fn labels_workspace_create_is_atomic_and_retains_failed_text_and_color(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        let target = install(shell);
        shell.labels_picker_open(target.clone(), Vec::new(), EditAccess::Editable, window, cx);
        let opening = shell.areas.labels.picker.as_ref().unwrap().opening;
        shell.labels_picker_create(opening, window, cx);
        let picker = shell.areas.labels.picker.as_mut().unwrap();
        let draft = picker.page.draft_mut().unwrap();
        assert_eq!(draft.color(), Color::Violet);
        draft.set_text("  New  label ");
        draft.set_color(Color::Blue);
        picker.name.set_text("  New  label ", window, cx);
        assert!(
            super::overlay(
                shell,
                &vessel_app_kit::Tokens::of(vessel_app_kit::Scheme::Dark),
                window,
                cx
            )
            .is_some()
        );
        shell.labels_picker_submit(opening, cx);
        let vessel_app_link::Command::Labels(vessel_app_link::labels::Request {
            context,
            ticket,
            ask:
                Command::Assign {
                    workspace,
                    label,
                    assignment,
                },
        }) = commands.try_recv().unwrap()
        else {
            panic!("one atomic create-and-assign");
        };
        assert_eq!(workspace, target.workspace());
        assert_eq!(label.name.as_str(), "New label");
        assert_eq!(label.color, Color::Blue);
        assert_eq!(assignment, Assignment::Assigned);
        shell.labels_picker_submit(opening, cx);
        assert!(commands.try_recv().is_err());
        shell.labels_answer(
            Event {
                context,
                ticket,
                answer: Answer::Assigned {
                    workspace,
                    name: label.name,
                    result: Err(Failure::Update),
                },
            },
            cx,
        );
        let picker = shell.areas.labels.picker.as_mut().unwrap();
        let draft = picker.page.draft_mut().unwrap();
        assert_eq!(draft.text(), "  New  label ");
        assert_eq!(draft.color(), Color::Blue);
        assert_eq!(draft.error(), Some(Failure::Update));
        assert_eq!(picker.name.text(cx), "  New  label ");
        shell.labels_picker_close(opening, window, cx);
        assert!(
            super::overlay(
                shell,
                &vessel_app_kit::Tokens::of(vessel_app_kit::Scheme::Dark),
                window,
                cx
            )
            .is_none()
        );
    });
}

#[gpui::test]
fn labels_workspace_create_success_reopens_list_without_claiming_assignment_state(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        let target = install(shell);
        shell.labels_picker_open(target.clone(), Vec::new(), EditAccess::Editable, window, cx);
        let opening = shell.areas.labels.picker.as_ref().unwrap().opening;
        shell.labels_picker_create(opening, window, cx);
        shell
            .areas
            .labels
            .picker
            .as_mut()
            .unwrap()
            .page
            .draft_mut()
            .unwrap()
            .set_text("New");
        shell.labels_picker_submit(opening, cx);
        let vessel_app_link::Command::Labels(vessel_app_link::labels::Request {
            context,
            ticket,
            ask: Command::Assign {
                workspace, label, ..
            },
        }) = commands.try_recv().unwrap()
        else {
            panic!("create and assignment share one operation");
        };
        shell.labels_answer(
            Event {
                context,
                ticket,
                answer: Answer::Assigned {
                    workspace,
                    name: label.name,
                    result: Ok(()),
                },
            },
            cx,
        );
        assert!(
            shell
                .areas
                .labels
                .picker
                .as_ref()
                .unwrap()
                .return_to_list
                .get()
        );
        let _element = super::overlay(
            shell,
            &vessel_app_kit::Tokens::of(vessel_app_kit::Scheme::Dark),
            window,
            cx,
        );
        assert!(
            !shell
                .areas
                .labels
                .picker
                .as_ref()
                .unwrap()
                .return_to_list
                .get()
        );
    });
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        assert!(shell.areas.labels.picker_combo.is_open());
        let picker = shell.areas.labels.picker.as_ref().unwrap();
        assert!(matches!(picker.page, PickerPage::Listing));
        assert!(picker.assigned.is_empty());
        let opening = picker.opening;
        shell.labels_picker_close(opening, window, cx);
    });
}

#[gpui::test]
fn labels_replaced_create_field_cannot_deliver_queued_text_to_its_successor(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let target = install(shell);
        shell.labels_picker_open(target.clone(), Vec::new(), EditAccess::Editable, window, cx);
        let first = shell.areas.labels.picker.as_ref().unwrap().opening;
        shell.labels_picker_create(first, window, cx);
        let source = shell.areas.labels.picker.as_ref().unwrap().name.id();
        cx.defer_in(window, move |shell, _window, cx| {
            super::field_changed(shell, source, "stale editor text".to_owned(), cx)
        });
        shell.labels_picker_open(target, Vec::new(), EditAccess::Editable, window, cx);
        let next = shell.areas.labels.picker.as_ref().unwrap().opening;
        shell.labels_picker_create(next, window, cx);
        assert_ne!(
            source,
            shell.areas.labels.picker.as_ref().unwrap().name.id()
        );
    });
    cx.run_until_parked();
    shell.update(cx, |shell, _cx| {
        let PickerPage::Creating(draft) = &shell.areas.labels.picker.as_ref().unwrap().page else {
            panic!("the new create draft remains idle");
        };
        assert_eq!(draft.text(), "");
    });
}
