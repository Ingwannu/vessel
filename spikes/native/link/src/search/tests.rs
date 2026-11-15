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
