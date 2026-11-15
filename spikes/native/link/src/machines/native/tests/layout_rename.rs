//! Framed RPC tests prove rename routing without starting a node or agent.
use super::peer;
use crate::workspace::layout::{Ask, run};
use std::time::Duration;
use vessel_app_view::layout::{HostAlias, RenameProblem, RenameSubmission};
use vessel_app_view::layout::{RenameTarget, RenameTicket};
use vessel_proto::{Call, ErrorCode, SessionId, TerminalId};
const TEST_DEADLINE: Duration = Duration::from_secs(5);
async fn refused_rename(target: RenameTarget) -> (Ask, Vec<Call>) {
    let mut peer = peer::connect("rename-test").await;
    let client = peer.client.take().unwrap();
    let ask = Ask {
        node: client.node().id,
        machine: HostAlias::parse("rename-test").unwrap(),
        submission: RenameSubmission {
            ticket: RenameTicket::ZERO.next().unwrap(),
            target,
            title: "renamed entity".parse().unwrap(),
        },
    };
    let answer = tokio::time::timeout(TEST_DEADLINE, run(&client, ask.clone()))
        .await
        .unwrap();
    assert_eq!(answer.ask.submission, ask.submission);
    assert_eq!(
        answer.result,
        Err(RenameProblem::Node(ErrorCode::Unsupported))
    );
    assert!(
        peer.subscribed.try_recv().is_err(),
        "rename must not replace the session stream"
    );
    let calls = peer.calls.lock().unwrap().clone();
    (ask, calls)
}
#[tokio::test]
async fn agent_rename_uses_the_agent_request_and_preserves_refusal_and_ticket() {
    let session = SessionId::random();
    let (ask, calls) = refused_rename(RenameTarget::Agent(session)).await;
    assert!(
        matches!(calls.as_slice(),[Call::RenameSession(request)] if request.session==session&&request.title==ask.submission.title)
    );
}
#[tokio::test]
async fn terminal_rename_uses_standalone_terminal_request_without_agent_lifecycle() {
    let terminal = TerminalId::random();
    let (ask, calls) = refused_rename(RenameTarget::Terminal(terminal)).await;
    assert!(
        matches!(calls.as_slice(),[Call::RenameTerminal(request)] if request.terminal==terminal&&request.title==ask.submission.title)
    );
}

#[tokio::test]
async fn renamed_alias_cannot_send_to_a_replacement_node() {
    let mut peer = peer::connect("rename-test").await;
    let client = peer.client.take().unwrap();
    let ask = Ask {
        node: vessel_proto::NodeId::random(),
        machine: HostAlias::parse("rename-test").unwrap(),
        submission: RenameSubmission {
            ticket: RenameTicket::ZERO.next().unwrap(),
            target: RenameTarget::Agent(SessionId::random()),
            title: "kept input".parse().unwrap(),
        },
    };
    assert_eq!(
        run(&client, ask).await.result,
        Err(RenameProblem::NodeChanged)
    );
    assert!(peer.calls.lock().unwrap().is_empty());
}
