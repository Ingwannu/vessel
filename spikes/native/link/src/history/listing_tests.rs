//! History pages retain provider identity, account scope and opaque continuation.

use super::*;
use vessel_proto::{
    ProtocolVersion, SessionId,
    history::{AgentHistoryPage, HistoryEntry},
};

#[tokio::test]
async fn history_uses_the_selected_account_folder_and_unmodified_cursor() {
    crate::testing::within(async {
        let existing = SessionId::random();
        let entry = HistoryEntry {
            harness_session: HarnessSessionId::new("native"),
            cwd: "/work/original".into(),
            title: Some("Kept title".parse().unwrap()),
            updated_at: Some(chrono::DateTime::UNIX_EPOCH),
            in_vessel: Some(existing),
        };
        let peer = peer::connect(vec![peer::Behaviour::Page(AgentHistoryPage {
            conversations: vec![entry.clone()],
            next: Some("provider:next".into()),
        })])
        .await;
        let mut request = batch(peer.client.node().id);
        request.scope.cwd = Some("/work/selected".into());
        let Reply::Listed {
            scope,
            ticket,
            result,
        } = bounded_answer(
            &peer::admitted(&peer).2,
            Ask::List {
                scope: request.scope.clone(),
                ticket: request.ticket,
                cursor: Some(Cursor::parse("opaque:input".into()).unwrap()),
            },
        )
        .await
        else {
            panic!("history receipt")
        };
        assert_eq!(scope, request.scope);
        assert_eq!(ticket, request.ticket);
        let listed = result.unwrap();
        assert_eq!(listed.node, peer.client.node().id);
        assert_eq!(listed.next.unwrap().as_str(), "provider:next");
        assert_eq!(listed.entries.len(), 1);
        let projected = listed.entries.first().unwrap();
        assert_eq!(projected.id, entry.harness_session);
        assert_eq!(projected.cwd, entry.cwd);
        assert_eq!(projected.title, entry.title);
        assert_eq!(projected.updated_at, entry.updated_at);
        assert_eq!(projected.in_vessel, Some(existing));
        {
            let calls = peer.calls.lock().unwrap();
            let Some(Call::GetAgentHistory(call)) = calls.first() else {
                panic!("agent.history")
            };
            assert_eq!(call.harness, scope.harness);
            assert_eq!(call.profile, scope.profile);
            assert_eq!(call.cwd, scope.cwd);
            assert_eq!(call.cursor.as_deref(), Some("opaque:input"));
        }

        peer.finish().await;
    })
    .await;
}

#[tokio::test]
async fn an_empty_filtered_page_can_still_offer_another_page() {
    crate::testing::within(async {
        let peer = peer::connect(vec![peer::Behaviour::Page(AgentHistoryPage {
            conversations: vec![],
            next: Some("continue".into()),
        })])
        .await;
        let request = batch(peer.client.node().id);
        let Reply::Listed { result, .. } = bounded_answer(
            &peer::admitted(&peer).2,
            Ask::List {
                scope: request.scope,
                ticket: request.ticket,
                cursor: None,
            },
        )
        .await
        else {
            panic!("history receipt")
        };
        let listed = result.unwrap();
        assert!(listed.entries.is_empty());
        assert_eq!(listed.next.unwrap().as_str(), "continue");

        peer.finish().await;
    })
    .await;
}

#[tokio::test]
async fn old_peer_is_refused_without_sending_an_unknown_history_method() {
    crate::testing::within(async {
        let peer = peer::connect_at(vec![], ProtocolVersion::new(16)).await;
        let request = batch(peer.client.node().id);
        let Reply::Listed { result, .. } = bounded_answer(
            &peer::admitted(&peer).2,
            Ask::List {
                scope: request.scope,
                ticket: request.ticket,
                cursor: None,
            },
        )
        .await
        else {
            panic!("history receipt")
        };
        assert_eq!(result, Err(Problem::Unsupported));
        assert!(peer.calls.lock().unwrap().is_empty());

        peer.finish().await;
    })
    .await;
}

#[test]
fn a_remote_page_cannot_persist_an_oversized_continuation() {
    let page = AgentHistoryPage {
        conversations: vec![],
        next: Some("x".repeat(Cursor::MAX_BYTES.saturating_add(1))),
    };
    assert_eq!(
        project(vessel_proto::NodeId::random(), page),
        Err(Problem::InvalidReply)
    );
}

async fn bounded_answer(selected: &crate::connections::SelectedConnection, request: Ask) -> Reply {
    crate::testing::phase(
        "history adapter result",
        crate::history::answer(selected, request),
    )
    .await
}
