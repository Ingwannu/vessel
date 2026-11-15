//! Real framed requests with synthetic replies; never a node process or browser.
use super::super::scripts::peer;
use super::*;
use vessel_app_view::workspace::git::RequestKeys;
use vessel_proto::{Call, ErrorCode, ProjectId, Reply, WireError};
fn commands() -> ProjectCommands {
    ProjectCommands::Listed {
        setup: vec![],
        teardown: vec![],
        scripts: vec![],
        terminals: vec![],
        service_ports: None,
        digest: "a".repeat(64).parse().unwrap(),
        approved: false,
        last_approved: None,
    }
}
#[tokio::test]
async fn reads_and_grants_echo_the_producing_admission_and_exact_digest() {
    let peer = peer::connect(|call| match call {
        Call::ProjectCommands(_) => Ok(Reply::ProjectCommands(commands())),
        Call::ApproveCommands(_) => Err(WireError::new(
            ErrorCode::Forbidden,
            "synthetic-approval-peer-diagnostic",
        )),
        other => panic!("unexpected request: {other:?}"),
    })
    .await;
    let (client, _connected, context) = peer.selected("fixture");
    let target = Target {
        context,
        project: ProjectId::random(),
    };
    let mut keys = RequestKeys::default();
    let read = keys.issue().unwrap();
    let answer = run(
        &client,
        Ask {
            target: target.clone(),
            key: read,
            operation: Operation::Read,
        },
    )
    .await;
    assert_eq!(answer.ask.target, target);
    assert_eq!(answer.ask.key, read);
    assert!(answer.result.is_ok());
    let digest: CommandsDigest = "a".repeat(64).parse().unwrap();
    let write = keys.issue().unwrap();
    let answer = run(
        &client,
        Ask {
            target: target.clone(),
            key: write,
            operation: Operation::Approve {
                digest: digest.clone(),
                approved: true,
            },
        },
    )
    .await;
    assert_eq!(answer.ask.target, target);
    assert_eq!(answer.ask.key, write);
    assert_eq!(
        answer.result,
        Err(Problem::Node {
            code: ErrorCode::Forbidden,
            message: String::new()
        })
    );
    let calls = peer.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert!(
        matches!(calls.first(), Some(Call::ProjectCommands(ask)) if ask.project == target.project && ask.base.is_none())
    );
    assert!(
        matches!(calls.last(), Some(Call::ApproveCommands(ask)) if ask.project == target.project && ask.digest == digest && ask.approved)
    );
}
#[tokio::test]
async fn retirement_before_poll_cannot_approve_a_replacement_connection() {
    let peer = peer::connect(|call| panic!("retired approval sent: {call:?}")).await;
    let (client, connected, context) = peer.selected("fixture");
    let key = RequestKeys::default().issue().unwrap();
    let request = Ask {
        target: Target {
            context,
            project: ProjectId::random(),
        },
        key,
        operation: Operation::Approve {
            digest: "a".repeat(64).parse().unwrap(),
            approved: true,
        },
    };
    let future = run(&client, request);
    connected.retire();
    assert!(matches!(future.await.result, Err(Problem::Gone(_))));
    assert!(peer.calls.lock().unwrap().is_empty());
}
