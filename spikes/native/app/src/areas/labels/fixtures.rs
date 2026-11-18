//! A followed host for tests outside the area: a session row whose
//! registry workspace has a label, as the host's first reads leave it.

use vessel_app_kit::ui::App;
use vessel_app_link::labels::{Answer, Event, Request};
use vessel_app_view::connection::{ConnectionContext, ConnectionEpochs, MachineKey};
use vessel_app_view::labels::{Catalog, Color, Host, HostState, Label};
use vessel_app_view::machine_book;
use vessel_proto::registry::{
    RegistryRevision, RegistrySnapshot, WorkspaceMembership, WorkspaceResource,
};
use vessel_proto::workspace_labels::{
    CatalogCursor, CatalogList, CatalogMode, WorkspaceLabelAssignments,
};
use vessel_proto::{NodeId, SessionId, WorkspaceId, WorkspaceLabelGeneration, WorkspaceLabelSeq};

use crate::shell::Shell;

/// The commands a test link sent.
pub(crate) type Sent = tokio::sync::mpsc::Receiver<vessel_app_link::Command>;

/// A host on protocol 28 that has not loaded yet, as a bind leaves it.
pub(crate) fn install(shell: &mut Shell) -> ConnectionContext {
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
pub(crate) fn sent(commands: &mut Sent) -> Request {
    let Ok(vessel_app_link::Command::Labels(request)) = commands.try_recv() else {
        panic!("expected a labels request");
    };
    request
}

/// A registry placing `session` in `workspace`, labelled Ready.
pub(crate) fn registry(session: SessionId, workspace: WorkspaceId) -> RegistrySnapshot {
    RegistrySnapshot {
        labels: vec![WorkspaceLabelAssignments {
            workspace,
            labels: vec!["Ready".parse().unwrap()],
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

/// Machine "fixture" followed, its catalog and registry read (online):
/// `session`'s row is in `workspace`, labelled Ready, and its Labels item
/// opens the picker.
pub(crate) fn followed_session(
    shell: &mut Shell,
    commands: &mut Sent,
    session: SessionId,
    workspace: WorkspaceId,
    cx: &mut gpui::Context<'_, Shell>,
) -> ConnectionContext {
    let connection = install(shell);
    shell.labels_follow(cx);
    let list = sent(commands);
    let read = sent(commands);
    let ready = Label {
        name: "Ready".parse().unwrap(),
        color: Color::Sky,
    };
    shell.labels_answer(
        Event {
            context: connection.clone(),
            ticket: list.ticket,
            answer: Answer::Listed(Ok(CatalogList {
                data: CatalogMode::Snapshot {
                    labels: vec![ready],
                },
                head: CatalogCursor {
                    generation: WorkspaceLabelGeneration::random(),
                    after: WorkspaceLabelSeq::new(1),
                },
            })),
        },
        cx,
    );
    shell.labels_answer(
        Event {
            context: connection.clone(),
            ticket: read.ticket,
            answer: Answer::Registry(Ok(Box::new(registry(session, workspace)))),
        },
        cx,
    );
    connection
}

/// What is typed in the open picker's search.
pub(crate) fn picker_query(shell: &Shell, cx: &App) -> String {
    shell.areas.labels.picker_combo.query(cx)
}

/// The host's answer to `request`, an assignment that made its label.
pub(crate) fn assigned(request: Request) -> Event {
    let vessel_app_link::labels::Command::Assign {
        workspace, label, ..
    } = request.ask
    else {
        panic!("create and assignment share one operation");
    };
    Event {
        context: request.context,
        ticket: request.ticket,
        answer: Answer::Assigned {
            workspace,
            name: label.name,
            result: Ok(Vec::new()),
        },
    }
}
