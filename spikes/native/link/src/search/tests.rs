use super::*;

#[tokio::test]
async fn a_silent_machine_settles_as_a_typed_timeout() {
    let result = bounded(std::future::pending(), std::time::Duration::ZERO).await;
    assert_eq!(result, Err(Refusal::TimedOut));
}

#[test]
fn peer_error_text_cannot_enter_the_palette_failure() {
    let canary = "synthetic-private-connection-invitation";
    let error = CallError::Remote(vessel_proto::WireError::new(ErrorCode::Forbidden, canary));
    let projected = refusal(&error);
    assert_eq!(
        projected,
        Refusal::Failed {
            code: Some(ErrorCode::Forbidden)
        }
    );
    assert!(!format!("{projected:?}").contains(canary));
    assert_eq!(refusal(&CallError::Disconnected), Refusal::NotConnected);
}

#[tokio::test]
async fn unknown_admission_settles_the_original_query_without_a_client() {
    let home = crate::testing::Home::new();
    let (state, mut events) = crate::testing::state(&home);
    state.search(Ask {
        machine: "fixture".to_owned(),
        connection: None,
        ticket: 19,
        request: SearchMessages {
            query: "needle".parse().unwrap(),
            session: None,
            before: None,
            limit: None,
        },
    });
    let Event::Searched(answer) = events.recv().await.unwrap() else {
        panic!("wrong event");
    };
    assert_eq!(answer.machine, "fixture");
    assert_eq!(answer.ticket, 19);
    assert_eq!(answer.result, Err(Refusal::NotConnected));
}

fn request(context: ConnectionContext) -> Ask {
    Ask {
        machine: context.machine().as_str().to_owned(),
        connection: Some(context),
        ticket: 23,
        request: SearchMessages {
            query: "needle".parse().unwrap(),
            session: None,
            before: None,
            limit: None,
        },
    }
}

fn admit(state: &mut State, client: vessel_client::NodeClient) -> crate::Connected {
    use vessel_app_view::connection::MachineKey;
    let context = state
        .admissions
        .admit(MachineKey::from_book("fixture"), client.node().id)
        .unwrap();
    let binding = vessel_client::book::BookEntry {
        address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
        token: None,
        credential: None,
        id: None,
    };
    let connected = crate::Connected::new(client, Vec::new(), binding, context);
    assert!(
        state
            .clients
            .insert("fixture".to_owned(), connected.clone())
            .is_none()
    );
    connected
}

#[tokio::test]
async fn retiring_before_first_poll_sends_no_search_frame() {
    use crate::testing::{Home, peer, phase, state, within};
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = peer::connect("fixture").await;
        let connected = admit(&mut state, peer.client.take().unwrap());
        state.search(request(connected.context.clone()));
        connected.retire(); // no await between selection and retirement
        let Event::Searched(answer) = phase("retired search answer", events.recv()).await.unwrap()
        else {
            panic!("wrong event");
        };
        assert_eq!(answer.ticket, 23);
        assert_eq!(answer.result, Err(Refusal::NotConnected));
        assert!(peer.calls.lock().unwrap().is_empty());
    })
    .await;
}

#[tokio::test]
async fn retirement_fences_an_in_flight_search_reply() {
    use crate::testing::{Home, peer, phase, state, within};
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = peer::controlled("fixture").await;
        let connected = admit(&mut state, peer.client.take().unwrap());
        let mut requests = peer.requests.take().unwrap();
        state.search(request(connected.context.clone()));
        let pending = phase("original search request", requests.recv())
            .await
            .unwrap();
        assert!(matches!(
            pending.call,
            vessel_proto::Call::SearchMessages(_)
        ));
        connected.retire();
        let _closed = pending.answer.send(vessel_proto::Outcome::Ok(
            vessel_proto::Reply::SearchMessages(SearchPage {
                hits: Vec::new(),
                next: None,
                reached: None,
            }),
        ));
        let Event::Searched(answer) = phase("fenced search reply", events.recv()).await.unwrap()
        else {
            panic!("wrong event");
        };
        assert_eq!(answer.machine, "fixture");
        assert_eq!(answer.ticket, 23);
        assert_eq!(answer.result, Err(Refusal::NotConnected));
        assert_eq!(peer.calls.lock().unwrap().len(), 1, "no retry or retarget");
    })
    .await;
}
