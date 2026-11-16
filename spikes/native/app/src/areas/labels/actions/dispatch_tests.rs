//! Actual shell dispatch with the detached link; no node, configuration or I/O.

use super::*;
use gpui::TestAppContext;
use vessel_app_link::Link;
use vessel_app_view::{
    connection::{ConnectionEpochs, MachineKey},
    labels::{Catalog, Color, Count, Host, HostState, Label},
    machine_book,
};

const COMMAND_CAPACITY: usize = 1;

fn contexts() -> (ConnectionContext, ConnectionContext) {
    let mut epochs = ConnectionEpochs::default();
    let node = HostKey::random();
    let first = epochs
        .admit(MachineKey::from_book("fixture"), node)
        .unwrap();
    let next = epochs
        .admit(MachineKey::from_book("fixture"), node)
        .unwrap();
    (first, next)
}

fn edit(shell: &mut Shell, context: &ConnectionContext) -> Target {
    let state = &mut shell.areas.labels;
    let opening = super::super::surface::Opening::new(state.tickets.issue().unwrap());
    let route = machine_book::Name::parse(context.machine().as_str()).unwrap();
    let label = Label {
        name: "Ready".parse().unwrap(),
        color: Color::Violet,
    };
    state.opening = Some(opening);
    state.route = Some(route.clone());
    state.hosts = vec![Host {
        key: context.node(),
        route,
        connection: Some(context.clone()),
        state: HostState::Online,
        catalog: Catalog::snapshot([label.clone()]),
    }];
    state.manager.select(context.node()).unwrap();
    state.manager.edit(label.clone()).unwrap();
    state
        .manager
        .draft_mut()
        .unwrap()
        .set_text("Retained draft");
    Target::new(opening, context.node(), Some(context.clone()), label.name)
}

#[gpui::test]
fn labels_shell_fences_stale_callbacks_and_settles_original_retired_requests(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    let (first, next) = contexts();
    shell.update(cx, |shell, cx| {
        let old = edit(shell, &first);
        shell.areas.labels.hosts.first_mut().unwrap().connection = Some(next.clone());
        shell.label_action(&Action::Save(old), cx);
        assert!(commands.try_recv().is_err());
        assert!(shell.areas.labels.pending.is_none());
        assert!(matches!(
            shell.areas.labels.manager.state(),
            ManagerState::Editing(_)
        ));
        let current = Target::new(
            shell.areas.labels.opening.unwrap(),
            next.node(),
            Some(next.clone()),
            "Ready".parse().unwrap(),
        );
        shell.label_action(&Action::Save(current), cx);
        let vessel_app_link::Command::Labels(Request {
            context,
            ticket,
            ask: Command::Update { .. },
        }) = commands.try_recv().unwrap()
        else {
            panic!("only the fresh click sends an atomic update");
        };
        assert_eq!(context, next);
        shell.labels_answer(
            Event {
                context: first,
                ticket,
                answer: Answer::Updated(Err(Failure::Changed)),
            },
            cx,
        );
        assert!(matches!(
            shell.areas.labels.manager.state(),
            ManagerState::Saving { .. }
        ));
        shell.labels_answer(
            Event {
                context: next.clone(),
                ticket,
                answer: Answer::Deleted(Err(Failure::Changed)),
            },
            cx,
        );
        assert!(shell.areas.labels.pending.is_some());
        // A transport retirement must not prevent its original refusal from
        // settling; it grants no authority to the next connection or catalog.
        shell.areas.labels.hosts.first_mut().unwrap().connection = None;
        shell.labels_answer(
            Event {
                context: next,
                ticket,
                answer: Answer::Updated(Err(Failure::Changed)),
            },
            cx,
        );
        assert!(shell.areas.labels.pending.is_none());
        assert_eq!(
            shell.areas.labels.manager.state().draft().unwrap().text(),
            "Retained draft"
        );
        assert_eq!(shell.areas.labels.manager.error(), Some(Failure::Changed));
        assert!(commands.try_recv().is_err());
    });
}

#[gpui::test]
fn labels_shell_keeps_inspection_and_final_delete_on_the_same_admission(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    let (first, next) = contexts();
    shell.update(cx, |shell, cx| {
        let target = edit(shell, &first);
        shell.label_action(&Action::Inspect(target.clone()), cx);
        let vessel_app_link::Command::Labels(Request {
            context,
            ticket,
            ask: Command::InspectDelete(name),
        }) = commands.try_recv().unwrap()
        else {
            panic!("inspection remains distinct from deletion");
        };
        assert_eq!(context, first);
        assert_eq!(name.as_str(), "Ready");
        shell.labels_answer(
            Event {
                context: first.clone(),
                ticket,
                answer: Answer::Inspected(Ok(Count::new(2))),
            },
            cx,
        );
        assert_eq!(shell.areas.labels.confirmation, Some(first));
        shell.areas.labels.hosts.first_mut().unwrap().connection = Some(next);
        shell.label_action(
            &Action::Delete {
                target,
                inspection: ticket,
            },
            cx,
        );
        assert!(commands.try_recv().is_err());
        assert!(matches!(
            shell.areas.labels.manager.state(),
            ManagerState::Confirming { .. }
        ));
        assert!(shell.areas.labels.manager.cancel_delete());
        assert_eq!(
            shell.areas.labels.manager.state().draft().unwrap().text(),
            "Retained draft"
        );
    });
}
