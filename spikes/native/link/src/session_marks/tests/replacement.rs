//! Alias replacement and a new admission of the SAME physical client both refuse.
use super::*;
use crate::{
    State,
    testing::{Home, phase},
};
fn admit(state: &mut State, client: vessel_client::NodeClient) -> crate::Connected {
    state
        .admit_connection(
            "fixture",
            &vessel_client::book::BookEntry {
                address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
                token: None,
                credential: None,
                id: Some(client.node().id),
            },
            client,
            vec![],
        )
        .unwrap()
}
#[tokio::test]
async fn stale_row_cannot_send_to_another_node_or_new_same_node_admission() {
    within(async {
        let mut old = peer::controlled("old").await;
        let mut other = peer::controlled("other").await;
        let home = Home::new();
        let (mut state, mut events) = crate::testing::state(&home);
        let original = admit(&mut state, old.client.as_ref().unwrap().clone());
        let intent = ask(&original.context, SessionId::random(), Mark::Archive);
        let _before = state.clients.insert("fixture".to_owned(), original);
        for client in [
            other.client.as_ref().unwrap().clone(),
            old.client.as_ref().unwrap().clone(),
        ] {
            state.clients.get("fixture").unwrap().retire();
            let new = admit(&mut state, client);
            let _replaced = state.clients.insert("fixture".to_owned(), new);
            state.handle(Command::SessionMark(intent.clone())).await;
            let Event::SessionMarked(answer) = phase("preflight refusal event", events.recv())
                .await
                .unwrap()
            else {
                panic!("mark")
            };
            assert!(matches!(
                answer.into_reply().into_parts().1,
                Err(Failure::Changed)
            ));
            assert!(old.calls.lock().unwrap().is_empty());
            assert!(other.calls.lock().unwrap().is_empty());
        }
        for peer in [&mut old, &mut other] {
            let task = peer.task.take().unwrap();
            task.abort();
            let _ended = phase("peer cancellation", task).await;
            phase(
                "physical transport close",
                peer.client.as_ref().unwrap().closed(),
            )
            .await;
        }
    })
    .await;
}
