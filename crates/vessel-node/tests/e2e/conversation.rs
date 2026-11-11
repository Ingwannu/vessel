//! Protocol 10 over the wire (milestone 11, track A): a session's
//! conversation as one call, live, from the store, and refused where there
//! is none.

use vessel_proto::{ErrorCode, EventSeq, HarnessKind, SessionEvent, SessionStatus};
use vessel_rpc::CallError;

use crate::support::{Fixture, read_until, status_of, structured_spec, terminal_spec, within};

fn code<T: std::fmt::Debug>(result: Result<T, CallError>) -> ErrorCode {
    match result {
        Err(CallError::Remote(error)) => error.code,
        other => panic!("expected a refusal from the node, got {other:?}"),
    }
}

/// `session.conversation`: the agent's abilities and commands while it
/// runs; after the node's restart, what the store kept; a terminal
/// session has none.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_conversation_is_read_live_and_from_the_store() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let id = client
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap()
            .id;
        let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        // The commands come when the agent sends them, before or after it
        // runs.
        read_until(&mut stream, &mut seen, |record| {
            matches!(record.event, SessionEvent::CommandsChanged { .. })
        })
        .await;
        read_until(&mut stream, &mut seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        let live = client.conversation(id).await.unwrap();
        assert!(live.abilities.is_some_and(|abilities| abilities.steer));
        assert_eq!(live.commands.len(), 2);

        node.stop().await.unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let stored = client.conversation(id).await.unwrap();
        assert_eq!(stored.abilities, live.abilities, "kept by the store");
        assert!(stored.commands.is_empty(), "an ended session offers none");
        assert_eq!(
            code(client.conversation(vessel_proto::SessionId::random()).await),
            ErrorCode::NotFound
        );

        let terminal = client
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap()
            .id;
        assert_eq!(
            code(client.conversation(terminal).await),
            ErrorCode::Unsupported
        );
        node.stop().await.unwrap();
    })
    .await;
}
