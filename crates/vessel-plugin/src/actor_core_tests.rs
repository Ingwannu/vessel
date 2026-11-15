use super::*;
use tokio::sync::{Semaphore, watch};
use vessel_proto::{
    NodeId, RawJson,
    plugins::{HelperFailure, HelperProtocol},
};

const REQUEST_TEST_BUDGET: std::time::Duration = std::time::Duration::from_secs(10);

struct Fixture {
    core: Core,
    wire: mpsc::Receiver<NodeToHelper>,
    state: watch::Receiver<RuntimeState>,
    calls: Arc<Semaphore>,
    bytes: Arc<Semaphore>,
    address: PluginAddress,
}

impl Fixture {
    fn new(capacity: usize) -> Self {
        let address = PluginAddress {
            node: NodeId::random(),
            plugin: "test".parse().expect("plugin"),
            generation: RuntimeGeneration::random(),
        };
        let (writer, wire) = mpsc::channel(capacity);
        let (published, state) = watch::channel(RuntimeState::Starting);
        let mut core = Core::new(address.clone(), writer, published);
        core.receive(HelperToNode::Ready {
            protocol: HelperProtocol::V1,
            client: document(&["echo"]),
            hooks: HookSubscriptions::default(),
        })
        .expect("ready");
        Self {
            core,
            wire,
            state,
            calls: Arc::new(Semaphore::new(MAX_PENDING_CALLS)),
            bytes: Arc::new(Semaphore::new(1000)),
            address,
        }
    }

    fn request(
        &self,
    ) -> (
        Request,
        oneshot::Receiver<Result<HelperPayload, CallError>>,
        CancellationToken,
    ) {
        let (reply, receive) = oneshot::channel();
        let cancel = CancellationToken::new();
        let request = Request {
            address: self.address.clone(),
            call: HelperCall::Invoke {
                method: "echo".parse().expect("method"),
                input: payload(),
            },
            reply,
            deadline: tokio::time::Instant::now()
                .checked_add(REQUEST_TEST_BUDGET)
                .expect("deadline"),
            cancel: cancel.clone(),
            bytes: Arc::clone(&self.bytes).try_acquire_owned().expect("bytes"),
            count: Arc::clone(&self.calls).try_acquire_owned().expect("count"),
        };
        (request, receive, cancel)
    }

    fn issued(&mut self) -> InvocationId {
        let NodeToHelper::Request { request, .. } = self.wire.try_recv().expect("issued") else {
            panic!("request");
        };
        request
    }
}

fn document(methods: &[&str]) -> ClientDocument {
    ClientDocument::new(
        methods
            .iter()
            .map(|name| name.parse().expect("method"))
            .collect(),
        vec![],
        vec![],
    )
    .expect("graph")
}

fn payload() -> HelperPayload {
    HelperPayload::try_from(RawJson::empty_object()).expect("payload")
}

#[tokio::test]
async fn stale_missing_and_expired_calls_never_reach_the_helper() {
    let mut fixture = Fixture::new(8);
    let (mut stale, receive, _cancel) = fixture.request();
    stale.address.generation = RuntimeGeneration::random();
    fixture.core.admit(stale).expect("admission reply");
    assert_eq!(receive.await.expect("reply"), Err(CallError::Stale));
    let (mut unknown, receive, _cancel) = fixture.request();
    unknown.call = HelperCall::Invoke {
        method: "unknown".parse().expect("method"),
        input: payload(),
    };
    fixture.core.admit(unknown).expect("admission reply");
    assert_eq!(receive.await.expect("reply"), Err(CallError::Unregistered));
    let (mut expired, receive, _cancel) = fixture.request();
    expired.deadline = tokio::time::Instant::now();
    fixture.core.admit(expired).expect("admission reply");
    assert_eq!(receive.await.expect("reply"), Err(CallError::Timeout));
    assert!(fixture.wire.try_recv().is_err());
}

#[tokio::test]
async fn ready_reply_cannot_win_an_expired_deadline_or_release_live_admission() {
    let mut fixture = Fixture::new(8);
    let (request, receive, _cancel) = fixture.request();
    fixture.core.admit(request).expect("admit");
    let id = fixture.issued();
    fixture.core.pending.get_mut(&id).expect("pending").deadline = tokio::time::Instant::now();
    assert!(matches!(
        fixture.core.receive(HelperToNode::Result {
            request: id,
            output: payload(),
        }),
        Err(RuntimeError::CallTimeout)
    ));
    assert_eq!(fixture.bytes.available_permits(), 999);
    fixture.core.retire(CallError::Timeout);
    assert_eq!(receive.await.expect("reply"), Err(CallError::Timeout));
    assert_eq!(fixture.bytes.available_permits(), 999);
}

#[tokio::test]
async fn cancellation_retains_count_and_bytes_until_actual_settlement() {
    let mut fixture = Fixture::new(8);
    let (request, receive, cancel) = fixture.request();
    fixture.core.admit(request).expect("admit");
    let id = fixture.issued();
    cancel.cancel();
    fixture.core.cancellations().expect("cancel");
    assert_eq!(receive.await.expect("reply"), Err(CallError::Cancelled));
    assert_eq!(
        fixture.wire.try_recv().expect("cancel frame"),
        NodeToHelper::Cancel { request: id }
    );
    assert_eq!(
        fixture.calls.available_permits(),
        MAX_PENDING_CALLS.saturating_sub(1)
    );
    assert_eq!(fixture.bytes.available_permits(), 999);
    fixture
        .core
        .receive(HelperToNode::Error {
            request: id,
            kind: HelperFailure::Cancelled,
            diagnostic: "cancelled".parse().expect("diagnostic"),
        })
        .expect("settle");
    assert_eq!(fixture.calls.available_permits(), MAX_PENDING_CALLS);
    assert_eq!(fixture.bytes.available_permits(), 1000);
}

#[tokio::test]
async fn changed_declarations_revoke_pending_and_rotate_public_generation() {
    let mut fixture = Fixture::new(8);
    let (request, receive, _cancel) = fixture.request();
    fixture.core.admit(request).expect("admit");
    let id = fixture.issued();
    fixture
        .core
        .receive(HelperToNode::ClientChanged {
            client: document(&["echo", "new"]),
        })
        .expect("replace");
    assert_eq!(receive.await.expect("revoked"), Err(CallError::Stale));
    let RuntimeState::Running {
        address: current, ..
    } = fixture.state.borrow().clone()
    else {
        panic!("running");
    };
    assert_ne!(current.generation, fixture.address.generation);
    assert_eq!(
        fixture.wire.try_recv().expect("cancel"),
        NodeToHelper::Cancel { request: id }
    );
    fixture
        .core
        .receive(HelperToNode::Result {
            request: id,
            output: payload(),
        })
        .expect("late settlement");
    let (stale, receive, _cancel) = fixture.request();
    fixture.core.admit(stale).expect("refuse");
    assert_eq!(receive.await.expect("reply"), Err(CallError::Stale));
}

#[tokio::test]
async fn retired_replies_cannot_complete_another_request_and_future_ids_refuse() {
    let mut fixture = Fixture::new(8);
    let (first, receive, _cancel) = fixture.request();
    fixture.core.admit(first).expect("admit");
    let old = fixture.issued();
    fixture
        .core
        .receive(HelperToNode::Result {
            request: old,
            output: payload(),
        })
        .expect("result");
    assert!(receive.await.expect("reply").is_ok());
    let (second, mut receive, _cancel) = fixture.request();
    fixture.core.admit(second).expect("admit");
    let next = fixture.issued();
    fixture
        .core
        .receive(HelperToNode::Result {
            request: old,
            output: payload(),
        })
        .expect("duplicate");
    assert!(matches!(
        receive.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    fixture
        .core
        .receive(HelperToNode::Result {
            request: next,
            output: payload(),
        })
        .expect("result");
    assert!(receive.await.expect("reply").is_ok());
    let future = InvocationId::new(
        NonZeroU64::new(next.get().checked_add(1).expect("next")).expect("nonzero"),
    );
    assert!(matches!(
        fixture.core.receive(HelperToNode::Result {
            request: future,
            output: payload()
        }),
        Err(RuntimeError::Protocol(ProtocolViolation::UnissuedRequest))
    ));
}

#[tokio::test]
async fn control_queue_full_forces_retirement_without_releasing_live_permits() {
    let mut fixture = Fixture::new(1);
    let (request, receive, cancel) = fixture.request();
    fixture.core.admit(request).expect("admit");
    cancel.cancel();
    assert!(matches!(
        fixture.core.cancellations(),
        Err(RuntimeError::Protocol(ProtocolViolation::ControlQueueFull))
    ));
    assert_eq!(
        receive.await.expect("cancel reply"),
        Err(CallError::Cancelled)
    );
    fixture.core.retire(CallError::Closed);
    assert_eq!(*fixture.state.borrow(), RuntimeState::Stopped);
    assert_eq!(
        fixture.calls.available_permits(),
        MAX_PENDING_CALLS.saturating_sub(1)
    );
    drop(fixture.core);
    assert_eq!(fixture.calls.available_permits(), MAX_PENDING_CALLS);
}

#[test]
fn second_ready_and_counter_exhaustion_are_closed_protocol_failures() {
    let mut fixture = Fixture::new(8);
    assert!(matches!(
        fixture.core.receive(HelperToNode::Ready {
            protocol: HelperProtocol::V1,
            client: document(&["echo"]),
            hooks: HookSubscriptions::default()
        }),
        Err(RuntimeError::Protocol(ProtocolViolation::Ordering))
    ));
    fixture.core.high = u64::MAX;
    let (request, _receive, _cancel) = fixture.request();
    assert!(matches!(
        fixture.core.admit(request),
        Err(RuntimeError::Protocol(ProtocolViolation::IdentityExhausted))
    ));
    assert!(fixture.wire.try_recv().is_err());
}
