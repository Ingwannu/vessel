use super::*;
use crate::{
    Connected,
    testing::{Home, peer, state, within},
};
use vessel_app_view::{
    connection::MachineKey, machine_book::Name, workspace::projects::actions::ProjectActions,
};
use vessel_client::book::BookEntry;

fn binding() -> BookEntry {
    BookEntry {
        address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
        token: None,
        credential: None,
        id: None,
    }
}

fn change(context: &ConnectionContext) -> Operation {
    let mut actions = ProjectActions::default();
    Operation::Change(ProjectChange {
        key: actions
            .begin(
                Name::parse(context.machine().as_str()).unwrap(),
                context.node(),
            )
            .unwrap(),
        action: ProjectAction::Register(vessel_proto::workspace::AddProject {
            path: "/synthetic/project".into(),
            name: None,
        }),
    })
}

async fn response(events: &mut tokio::sync::mpsc::Receiver<crate::Event>) -> Response {
    loop {
        if let crate::Event::Projects(response) = events.recv().await.unwrap() {
            return response;
        }
    }
}

#[tokio::test]
async fn a_same_node_reconnect_refuses_the_old_admission_before_any_rpc() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = peer::connect("project-peer").await;
        let client = peer.client.take().unwrap();
        let old = state
            .admissions
            .admit(MachineKey::from_book("fixture"), client.node().id)
            .unwrap();
        let current = state
            .admissions
            .admit(MachineKey::from_book("fixture"), client.node().id)
            .unwrap();
        let _previous = state.clients.insert(
            "fixture".into(),
            Connected::new(client, Vec::new(), binding(), current),
        );
        state
            .projects(Request {
                operation: change(&old),
                context: old.clone(),
            })
            .await;
        let response = response(&mut events).await;
        assert_eq!(response.context, old);
        assert!(matches!(
            response.answer,
            Answer::ProjectChanged {
                result: Err(ProjectFailure::HostChanged),
                ..
            }
        ));
        assert!(peer.calls.lock().unwrap().is_empty());
    })
    .await;
}

#[tokio::test]
async fn retiring_a_dispatched_write_is_uncertain_and_is_never_replayed() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = peer::controlled("project-peer").await;
        let client = peer.client.take().unwrap();
        let context = state
            .admissions
            .admit(MachineKey::from_book("fixture"), client.node().id)
            .unwrap();
        let _previous = state.clients.insert(
            "fixture".into(),
            Connected::new(client, Vec::new(), binding(), context.clone()),
        );
        state
            .projects(Request {
                operation: change(&context),
                context: context.clone(),
            })
            .await;
        let request = peer.requests.as_mut().unwrap().recv().await.unwrap();
        assert!(matches!(request.call, vessel_proto::Call::AddProject(_)));
        state.retire_connections().await;
        let response = response(&mut events).await;
        assert_eq!(response.context, context);
        assert!(matches!(
            response.answer,
            Answer::ProjectChanged {
                result: Err(ProjectFailure::OutcomeUnknown),
                ..
            }
        ));
        assert_eq!(peer.calls.lock().unwrap().len(), 1);
        drop(request);
    })
    .await;
}

#[tokio::test]
async fn exhausted_project_capacity_refuses_without_sending_a_write() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = peer::connect("project-peer").await;
        let client = peer.client.take().unwrap();
        let context = state
            .admissions
            .admit(MachineKey::from_book("fixture"), client.node().id)
            .unwrap();
        let _previous = state.clients.insert(
            "fixture".into(),
            Connected::new(client, Vec::new(), binding(), context.clone()),
        );
        let permits = state
            .workspace
            .project_calls
            .permits
            .acquire_many(u32::try_from(MAX_PROJECT_CALLS).unwrap())
            .await
            .unwrap();
        state
            .projects(Request {
                operation: change(&context),
                context,
            })
            .await;
        assert!(matches!(
            response(&mut events).await.answer,
            Answer::ProjectChanged {
                result: Err(ProjectFailure::Call(Problem::Unsent)),
                ..
            }
        ));
        assert!(peer.calls.lock().unwrap().is_empty());
        drop(permits);
    })
    .await;
}
