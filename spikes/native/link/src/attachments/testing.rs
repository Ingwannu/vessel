//! The actual shared actor owns a test admission; no guessed epoch or registry.
use crate::{State, connections::SelectedConnection, testing::Home};
use vessel_app_view::connection::ConnectionContext;
use vessel_client::NodeClient;

pub(super) fn selected(client: NodeClient) -> (Home, State, ConnectionContext, SelectedConnection) {
    let home = Home::new();
    let (mut state, _events) = crate::testing::state(&home);
    let binding = vessel_client::book::BookEntry {
        address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
        token: None,
        credential: None,
        id: Some(client.node().id),
    };
    let connected = state
        .admit_connection("fixture", &binding, client, Vec::new())
        .unwrap();
    let context = connected.context.clone();
    let _old = state.clients.insert("fixture".to_owned(), connected);
    let selected = state.selected_connection(&context).unwrap();
    (home, state, context, selected)
}
