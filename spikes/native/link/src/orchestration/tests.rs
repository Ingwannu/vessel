use super::{Action, Output, Problem, calls, runner};
use runner::Node;
use std::sync::atomic::{AtomicUsize, Ordering};
use vessel_app_view::devices::{MachineKey, Tickets};
use vessel_app_view::orchestration::{Consumer, Scope, Target, Task, Visit};
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
fn task(id: NodeId) -> Task {
    let mut tickets = Tickets::default();
    Task {
        scope: Scope {
            consumer: Consumer::Onboarding,
            target: Target {
                machine: MachineKey::from_book("host"),
                node: id,
            },
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
