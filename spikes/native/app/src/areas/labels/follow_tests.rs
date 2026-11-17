//! The followed reads over the test link: what is asked, and what each
//! answer changes for the manager, the picker and the rows.

use std::time::Duration;

use gpui::TestAppContext;
use vessel_app_link::Link;
use vessel_app_link::labels::{Answer, Command, Event, Request};
use vessel_app_view::connection::{ConnectionContext, ConnectionEpochs, MachineKey};
use vessel_app_view::labels::{Catalog, Color, Host, HostState, Label, Name};
use vessel_app_view::machine_book;
use vessel_proto::registry::{
    RegistryRevision, RegistrySnapshot, WorkspaceMembership, WorkspaceResource,
};
use vessel_proto::workspace_labels::{
    CatalogCursor, CatalogList, CatalogMode, CatalogRead, WorkspaceLabelAssignments,
};
use vessel_proto::{NodeId, SessionId, WorkspaceId, WorkspaceLabelGeneration, WorkspaceLabelSeq};

use crate::shell::Shell;

const COMMAND_CAPACITY: usize = 8;

fn name(text: &str) -> Name {
    text.parse().unwrap()
}

fn ready() -> Label {
    Label {
        name: name("Ready"),
        color: Color::Sky,
    }
}

/// A host on protocol 28 that has not loaded yet, as a bind leaves it.
fn install(shell: &mut Shell) -> ConnectionContext {
    let connection = ConnectionEpochs::default()
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    shell.areas.labels.hosts = vec![Host {
        key: connection.node(),
        route: machine_book::Name::parse("fixture").unwrap(),
        connection: Some(connection.clone()),
        state: HostState::Loading,
        catalog: Catalog::default(),
    }];
    connection
}

/// The next command the area sent: a labels request.
fn sent(commands: &mut tokio::sync::mpsc::Receiver<vessel_app_link::Command>) -> Request {
    let Ok(vessel_app_link::Command::Labels(request)) = commands.try_recv() else {
        panic!("expected a labels request");
    };
    request
}

fn head() -> CatalogCursor {
    CatalogCursor {
        generation: WorkspaceLabelGeneration::random(),
        after: WorkspaceLabelSeq::new(1),
    }
}

fn registry(session: SessionId, workspace: WorkspaceId) -> RegistrySnapshot {
    RegistrySnapshot {
        labels: vec![WorkspaceLabelAssignments {
            workspace,
            labels: vec![name("Ready")],
        }],
        node: NodeId::random(),
        revision: RegistryRevision::ZERO.next().unwrap(),
        workspaces: Vec::new(),
        memberships: vec![WorkspaceMembership {
            workspace,
            resource: WorkspaceResource::Session { session },
        }],
    }
}

#[gpui::test]
fn a_capable_host_is_followed_and_its_answers_reach_the_rows(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    let session = SessionId::random();
    let workspace = WorkspaceId::random();
    let cursor = head();
    shell.update_in(cx, |shell, _window, cx| {
        let connection = install(shell);
        shell.labels_follow(cx);
        let list = sent(&mut commands);
        assert!(matches!(
            list.ask,
            Command::List {
                cursor: None,
                read: CatalogRead::Now
            }
        ));
        let read = sent(&mut commands);
        assert!(matches!(read.ask, Command::Registry { changed_from: None }));
        assert!(commands.try_recv().is_err(), "one read of each out");
        // An answer to another request changes nothing.
        let stale = Event {
            context: connection.clone(),
            ticket: shell.areas.labels.tickets.issue().unwrap(),
            answer: Answer::Listed(Ok(CatalogList {
                data: CatalogMode::Snapshot { labels: Vec::new() },
                head: cursor,
            })),
        };
        shell.labels_answer(stale, cx);
        assert_eq!(shell.areas.labels.hosts[0].state, HostState::Loading);
        shell.labels_answer(
            Event {
                context: connection.clone(),
                ticket: list.ticket,
                answer: Answer::Listed(Ok(CatalogList {
                    data: CatalogMode::Snapshot {
                        labels: vec![ready()],
                    },
                    head: cursor,
                })),
            },
            cx,
        );
        let host = &shell.areas.labels.hosts[0];
        assert_eq!(host.state, HostState::Online);
        assert_eq!(host.catalog.get(&name("Ready")), Some(&ready()));
        shell.labels_answer(
            Event {
                context: connection,
                ticket: read.ticket,
                answer: Answer::Registry(Ok(Box::new(registry(session, workspace)))),
            },
            cx,
        );
        let row = shell.areas.labels.row("fixture", session).unwrap();
        assert_eq!(row.workspace, workspace);
        assert_eq!(row.labels, [ready()]);
        assert!(row.writable);
        assert!(
            shell
                .areas
                .labels
                .row("fixture", SessionId::random())
                .is_none()
        );
    });
    // After the quiet pause, each read waits from where its answer left off.
    cx.executor().advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    let next = sent(&mut commands);
    let mut waiting = [next.ask, sent(&mut commands).ask];
    waiting.sort_by_key(|ask| matches!(ask, Command::Registry { .. }));
    assert!(matches!(
        waiting[0],
        Command::List {
            cursor: Some(from),
            read: CatalogRead::Observe
        } if from == cursor
    ));
    assert!(matches!(
        waiting[1],
        Command::Registry {
            changed_from: Some(_)
        }
    ));
}

#[gpui::test]
fn an_older_peer_stops_the_reads_and_says_unsupported(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, _window, cx| {
        let connection = install(shell);
        shell.labels_follow(cx);
        let list = sent(&mut commands);
        let _registry = sent(&mut commands);
        shell.labels_answer(
            Event {
                context: connection,
                ticket: list.ticket,
                answer: Answer::Listed(Err(vessel_app_view::labels::Failure::Unsupported)),
            },
            cx,
        );
        assert_eq!(shell.areas.labels.hosts[0].state, HostState::Unsupported);
        // The fleet binds the same admission again: still Unsupported, not
        // a Loading that nothing will settle.
        shell.areas.labels.hosts[0].state = HostState::Loading;
        shell.labels_follow(cx);
        assert_eq!(shell.areas.labels.hosts[0].state, HostState::Unsupported);
    });
    cx.executor().advance_clock(Duration::from_secs(30));
    cx.run_until_parked();
    assert!(commands.try_recv().is_err(), "no catalog read again");
}

#[gpui::test]
fn a_row_s_labels_item_opens_the_picker_on_its_registry_workspace(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    let session = SessionId::random();
    let workspace = WorkspaceId::random();
    shell.update_in(cx, |shell, window, cx| {
        let connection = install(shell);
        shell.labels_follow(cx);
        let _list = sent(&mut commands);
        let read = sent(&mut commands);
        shell.labels_answer(
            Event {
                context: connection.clone(),
                ticket: read.ticket,
                answer: Answer::Registry(Ok(Box::new(registry(session, workspace)))),
            },
            cx,
        );
        shell.labels_row_open("fixture", session, window, cx);
        let target = shell.areas.labels.picker_target().unwrap();
        assert_eq!(target.workspace(), workspace);
        assert_eq!(target.connection(), &connection);
        // A committed assignment shows on the row at once.
        let ticket = shell.areas.labels.tickets.issue().unwrap();
        shell.labels_answer(
            Event {
                context: connection,
                ticket,
                answer: Answer::Assigned {
                    workspace,
                    name: name("Blocked"),
                    result: Ok(vec![name("Ready"), name("Blocked")]),
                },
            },
            cx,
        );
        let row = shell.areas.labels.row("fixture", session).unwrap();
        let names: Vec<Name> = row.labels.into_iter().map(|label| label.name).collect();
        assert_eq!(names, [name("Ready"), name("Blocked")]);
    });
}

/// The host loaded (a snapshot at `cursor`) and its catalog read asked
/// again, waiting from `cursor`; the registry read left out.
fn loaded(
    shell: &gpui::Entity<Shell>,
    cx: &mut gpui::VisualTestContext,
    commands: &mut tokio::sync::mpsc::Receiver<vessel_app_link::Command>,
    cursor: CatalogCursor,
) -> (ConnectionContext, Request) {
    let connection = shell.update_in(cx, |shell, _window, cx| {
        let connection = install(shell);
        shell.labels_follow(cx);
        let list = sent(commands);
        let _registry = sent(commands);
        shell.labels_answer(
            Event {
                context: connection.clone(),
                ticket: list.ticket,
                answer: Answer::Listed(Ok(CatalogList {
                    data: CatalogMode::Snapshot {
                        labels: vec![ready()],
                    },
                    head: cursor,
                })),
            },
            cx,
        );
        connection
    });
    cx.executor().advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    (connection, sent(commands))
}

#[gpui::test]
fn a_gap_in_the_catalog_reads_a_snapshot_at_once_and_stays_loading(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    let cursor = head();
    let (connection, waiting) = loaded(&shell, cx, &mut commands, cursor);
    shell.update_in(cx, |shell, _window, cx| {
        // Changes from another cursor than the host's: a gap.
        let elsewhere = CatalogCursor {
            generation: WorkspaceLabelGeneration::random(),
            after: WorkspaceLabelSeq::new(7),
        };
        shell.labels_answer(
            Event {
                context: connection,
                ticket: waiting.ticket,
                answer: Answer::Listed(Ok(CatalogList {
                    data: CatalogMode::Changes {
                        from: elsewhere,
                        updates: Vec::new(),
                    },
                    head: elsewhere,
                })),
            },
            cx,
        );
        assert_eq!(shell.areas.labels.hosts[0].state, HostState::Loading);
    });
    // At once: no quiet pause before the snapshot.
    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert!(matches!(
        sent(&mut commands).ask,
        Command::List {
            cursor: None,
            read: CatalogRead::Now
        }
    ));
}

#[gpui::test]
fn reads_that_come_back_at_once_with_nothing_back_off(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    let cursor = head();
    let (connection, waiting) = loaded(&shell, cx, &mut commands, cursor);
    shell.update_in(cx, |shell, _window, cx| {
        // No wait share: the waiting read answers at once, unchanged.
        shell.labels_answer(
            Event {
                context: connection,
                ticket: waiting.ticket,
                answer: Answer::Listed(Ok(CatalogList {
                    data: CatalogMode::Changes {
                        from: cursor,
                        updates: Vec::new(),
                    },
                    head: cursor,
                })),
            },
            cx,
        );
    });
    // Not after the quiet second: the pause doubled.
    cx.executor().advance_clock(Duration::from_millis(1500));
    cx.run_until_parked();
    assert!(commands.try_recv().is_err());
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    assert!(matches!(
        sent(&mut commands).ask,
        Command::List {
            cursor: Some(_),
            read: CatalogRead::Observe
        }
    ));
}
