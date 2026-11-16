use super::*;
use vessel_proto::{
    NodeId,
    plugins::{ClientDocument, HelperToNode, HookSubscriptions, RuntimeGeneration},
};

fn ready() -> HelperToNode {
    HelperToNode::Ready {
        protocol: HelperProtocol::V1,
        client: ClientDocument::new(vec![], vec![], vec![]).expect("declarations"),
        hooks: HookSubscriptions::default(),
    }
}

#[tokio::test(start_paused = true)]
async fn ready_pipe_at_the_expired_initialization_deadline_never_publishes_capability() {
    let address = PluginAddress {
        node: NodeId::random(),
        plugin: "fixture".parse().expect("plugin"),
        generation: RuntimeGeneration::random(),
    };
    let (writer, _outgoing) = mpsc::channel(PIPE_QUEUE);
    let (published, state) = watch::channel(RuntimeState::Starting);
    let mut core = Core::new(address, writer, published);
    let deadline = tokio::time::Instant::now()
        .checked_add(REQUEST_DEADLINE)
        .expect("test deadline");
    let logs = SharedLogs::default();
    tokio::time::advance(REQUEST_DEADLINE).await;
    assert!(matches!(
        accept_message(&mut core, &logs, ready(), deadline),
        Err(RuntimeError::ReadyTimeout)
    ));
    assert!(!core.is_ready());
    assert_eq!(*state.borrow(), RuntimeState::Starting);
    assert!(logs.snapshot().expect("logs").is_empty());
}

#[tokio::test(start_paused = true)]
async fn timely_ready_activates_and_later_callbacks_do_not_reuse_the_initialization_deadline() {
    let address = PluginAddress {
        node: NodeId::random(),
        plugin: "fixture".parse().expect("plugin"),
        generation: RuntimeGeneration::random(),
    };
    let (writer, _outgoing) = mpsc::channel(PIPE_QUEUE);
    let (published, state) = watch::channel(RuntimeState::Starting);
    let mut core = Core::new(address, writer, published);
    let deadline = tokio::time::Instant::now()
        .checked_add(REQUEST_DEADLINE)
        .expect("test deadline");
    let logs = SharedLogs::default();
    accept_message(&mut core, &logs, ready(), deadline).expect("timely ready");
    assert!(matches!(*state.borrow(), RuntimeState::Running { .. }));
    tokio::time::advance(REQUEST_DEADLINE).await;
    accept_message(
        &mut core,
        &logs,
        HelperToNode::HooksChanged {
            hooks: HookSubscriptions::default(),
        },
        deadline,
    )
    .expect("initialization budget no longer governs callbacks");
    assert!(core.is_ready());
}
