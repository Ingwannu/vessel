use super::{Action, Output, Problem, calls, runner};
use runner::Node;
use std::sync::atomic::{AtomicUsize, Ordering};
use vessel_app_view::devices::{MachineKey, Tickets};
use vessel_app_view::orchestration::{Consumer, Scope, Task, Visit};
use vessel_proto::mcp::{Injection, McpSettings};
use vessel_proto::{ErrorCode, NodeId};

struct Fake {
    id: NodeId,
    supported: bool,
    hang: bool,
    calls: AtomicUsize,
}
impl Node for Fake {
    fn identity(&self) -> NodeId {
        self.id
    }
    fn supported(&self) -> bool {
        self.supported
    }
    async fn execute(&self, action: Action) -> Result<Output, Problem> {
        let _previous = self.calls.fetch_add(1, Ordering::SeqCst);
        if self.hang {
            return std::future::pending().await;
        }
        assert_eq!(action, Action::ReadTools);
        Ok(Output::Tools(McpSettings::default()))
    }
}
#[cfg(test)]
fn task(id: NodeId) -> Task {
    let mut tickets = Tickets::default();
    Task {
        scope: Scope {
            consumer: Consumer::Onboarding,
            target: vessel_app_view::connection::ConnectionEpochs::default()
                .admit(MachineKey::from_book("host"), id)
                .unwrap(),
            visit: Visit::new(tickets.next_ticket().unwrap()),
        },
        ticket: tickets.next_ticket().unwrap(),
        action: Action::ReadTools,
    }
}
#[tokio::test]
async fn wrong_identity_and_missing_feature_refuse_before_effects() {
    let mut node = Fake {
        id: NodeId::random(),
        supported: false,
        hang: false,
        calls: AtomicUsize::new(0),
    };
    assert_eq!(
        runner::run(&node, &task(NodeId::random())).await,
        Err(Problem::Changed)
    );
    assert_eq!(
        runner::run(&node, &task(node.id)).await,
        Err(Problem::Unsupported)
    );
    assert_eq!(node.calls.load(Ordering::SeqCst), 0);
    node.supported = true;
    assert!(matches!(
        runner::run(&node, &task(node.id)).await,
        Ok(Output::Tools(_))
    ));
    assert_eq!(node.calls.load(Ordering::SeqCst), 1);
}
#[tokio::test(start_paused = true)]
async fn deadline_is_unconfirmed_and_does_not_retry_a_mutation() {
    let node = Fake {
        id: NodeId::random(),
        supported: true,
        hang: true,
        calls: AtomicUsize::new(0),
    };
    let mut pending = task(node.id);
    pending.action = Action::Uninstall;
    assert_eq!(runner::run(&node, &pending).await, Err(Problem::TimedOut));
    assert_eq!(node.calls.load(Ordering::SeqCst), 1);
}
#[test]
fn injection_is_a_partial_host_patch_and_errors_carry_no_peer_text() {
    let patch = calls::injection(Injection::On);
    assert_eq!(patch.mcp.unwrap().inject, Some(Injection::On));
    assert!(patch.park_after.is_none());
    assert!(patch.drivers.is_none());
    let patch = calls::injection(Injection::Off).mcp.unwrap();
    assert!(patch.policy.is_none());
    let unsafe_text = "a credential-bearing peer diagnostic";
    let safe = calls::problem(vessel_client::CallError::Remote(
        vessel_proto::WireError::new(ErrorCode::Forbidden, unsafe_text),
    ));
    assert_eq!(safe, Problem::Refused(ErrorCode::Forbidden));
    assert!(!format!("{safe:?}").contains(unsafe_text));
}

#[tokio::test]
async fn retired_original_connection_cannot_poll_a_read_or_follow_a_new_alias() {
    use crate::testing::{Home, peer, state};
    let home = Home::new();
    let (mut actor, _events) = state(&home);
    let mut peer = peer::connect("host").await;
    let client = peer.client.take().unwrap();
    let context = actor
        .admissions
        .admit(MachineKey::from_book("host"), client.node().id)
        .unwrap();
    let binding = vessel_client::book::BookEntry {
        address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
        token: None,
        credential: None,
        id: None,
    };
    let connected = crate::Connected::new(client, Vec::new(), binding, context.clone());
    let _previous = actor.clients.insert("host".into(), connected);
    let selected = calls::Selected {
        connection: actor.selected_connection(&context).unwrap(),
        context: context.clone(),
    };
    // The async method is constructed before retirement and first polled
    // afterwards, exercising the actual shared guard rather than a mock flag.
    let pending = selected.execute(Action::ReadTools);
    let Some(original) = actor.clients.get("host") else {
        panic!("the original test connection is missing");
    };
    original.retire();
    assert_eq!(pending.await, Err(Problem::Changed));
    assert!(peer.calls.lock().unwrap().is_empty());
    assert!(actor.selected_connection(&context).is_err());
}
