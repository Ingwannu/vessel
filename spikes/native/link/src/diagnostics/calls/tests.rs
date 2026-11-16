//! Real framed RPC proves the diagnostic projection cannot execute discovery.
use super::*;
use std::num::NonZeroUsize;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream};
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Call, ClientFrame, ClientInfo, ErrorCode, NodeId, NodeInfo, NodeState, Outcome, ProtocolRange,
    Reply, ServerFrame, Welcome, WireError,
};
const FRAME_BYTES: usize = 65_536;
const CANARY: &str = "synthetic-private-value-never-export";
fn node(id: NodeId) -> NodeInfo {
    NodeInfo {
        id,
        name: CANARY.to_owned(),
        hostname: CANARY.to_owned(),
        os: CANARY.to_owned(),
        arch: CANARY.to_owned(),
        version: CANARY.to_owned(),
        protocol: ProtocolRange::CURRENT,
        started_at: chrono::DateTime::UNIX_EPOCH,
        state: NodeState::Serving,
        public_key: None,
        timezone: None,
    }
}
async fn read(stream: &mut DuplexStream) -> ClientFrame {
    let size = usize::try_from(stream.read_u32().await.unwrap()).unwrap();
    assert!(size <= FRAME_BYTES);
    let mut bytes = vec![0; size];
    assert_eq!(stream.read_exact(&mut bytes).await.unwrap(), bytes.len());
    serde_json::from_slice(&bytes).unwrap()
}
async fn write(stream: &mut DuplexStream, frame: ServerFrame) {
    let bytes = serde_json::to_vec(&frame).unwrap();
    stream
        .write_u32(u32::try_from(bytes.len()).unwrap())
        .await
        .unwrap();
    stream.write_all(&bytes).await.unwrap();
}
async fn peer(admitted: NodeId, outcome: Outcome) -> (NodeClient, tokio::task::JoinHandle<()>) {
    let (local, mut remote) = tokio::io::duplex(FRAME_BYTES);
    let task = tokio::spawn(async move {
        welcome(&mut remote, admitted).await;
        for _ in 0..2 {
            let ClientFrame::Request { id, call } = read(&mut remote).await else {
                panic!("expected diagnostic read");
            };
            let outcome = match call {
                Call::NodeInfo(_) => outcome.clone(),
                Call::Diagnostics(_) => {
                    Outcome::Err(WireError::new(ErrorCode::Unsupported, CANARY))
                }
                other => panic!("unexpected diagnostic request: {other:?}"),
            };
            write(&mut remote, ServerFrame::Response { id, outcome }).await;
        }
        let mut byte = [0];
        let _closed = remote.read(&mut byte).await;
    });
    (framed_client(local).await, task)
}
#[tokio::test]
async fn node_info_projects_only_allowed_fields_without_discovery() {
    let id = NodeId::random();
    let (client, task) = peer(id, Outcome::Ok(Reply::NodeInfo(node(id)))).await;
    let answer = collect_client(client).await;
    assert_eq!(answer.admitted, id);
    assert_eq!(answer.facts.as_ref().unwrap().node, id);
    assert!(!format!("{answer:?}").contains(CANARY));
    task.abort();
}
#[tokio::test]
async fn peer_error_message_never_crosses_the_diagnostic_edge() {
    for (code, expected) in [
        (ErrorCode::Forbidden, Problem::Forbidden),
        (ErrorCode::Unsupported, Problem::Unsupported),
        (ErrorCode::Internal, Problem::Failed),
    ] {
        let (client, task) =
            peer(NodeId::random(), Outcome::Err(WireError::new(code, CANARY))).await;
        let answer = collect_client(client).await;
        assert_eq!(answer.facts, Err(expected));
        assert!(!format!("{answer:?}").contains(CANARY));
        task.abort();
    }
}
#[test]
fn collector_capacity_and_queue_refusal_are_bounded() {
    let hub = Hub::default();
    let permits: Vec<_> = (0..MAX_PARALLEL_HOSTS)
        .map(|_| Arc::clone(&hub.0).try_acquire_owned().unwrap())
        .collect();
    assert!(Arc::clone(&hub.0).try_acquire_owned().is_err());
    drop(permits);
    assert!(Arc::clone(&hub.0).try_acquire_owned().is_ok());
    assert_eq!(super::super::rejected(crate::Rejected::Full), Problem::Busy);
    assert_eq!(
        super::super::rejected(crate::Rejected::Closed),
        Problem::Offline
    );
}

#[tokio::test]
async fn a_reply_for_another_node_is_not_attributed_to_the_admitted_host() {
    let admitted = NodeId::random();
    let (client, task) = peer(
        admitted,
        Outcome::Ok(Reply::NodeInfo(node(NodeId::random()))),
    )
    .await;
    let answer = collect_client(client).await;
    assert_eq!(answer.admitted, admitted);
    assert_eq!(answer.facts, Err(Problem::InvalidReply));
    task.abort();
}

#[tokio::test]
async fn a_fresh_framed_connection_notifies_before_an_identical_fleet() {
    use crate::diagnostics::Message;
    use vessel_app_view::devices::{MachineKey, Tickets};
    use vessel_app_view::diagnostics::collection::{Collection, Phase, Progress, ReportText};
    const LIFECYCLE_ROOM: usize = 4;
    let node_id = NodeId::random();
    let (client, first_task) = peer(node_id, Outcome::Ok(Reply::NodeInfo(node(node_id)))).await;
    let (replacement, second_task) =
        peer(node_id, Outcome::Ok(Reply::NodeInfo(node(node_id)))).await;
    let (events, mut receiving) = tokio::sync::mpsc::channel(LIFECYCLE_ROOM);
    let (_home, state) = fixture_state(events);
    let book_name = "laptop".parse::<vessel_client::book::NodeName>().unwrap();
    let machine = MachineKey::from_book(book_name.as_str());
    let mut tickets = Tickets::default();
    let ticket = tickets.next_ticket().unwrap();
    let mut collection = Collection::default();
    collection.start(
        ticket,
        ReportText::new("Client").unwrap(),
        [(machine.clone(), Ok(()))],
    );
    collection.running(ticket, &machine);
    let first = collect_client(client).await;
    collection.answer(ticket, &machine, Ok(first.clone()));
    assert_eq!(collection.phase(), &Phase::Ready);
    let fleet = vessel_app_view::Fleet::default();
    state.connection_ready(&book_name, &replacement).await;
    state.emit(crate::Event::Fleet(fleet.clone())).await;
    let crate::Event::Diagnostics(Message::Connected {
        machine: observed,
        node,
    }) = receiving.recv().await.unwrap()
    else {
        panic!("the new connection must have its own lifecycle notice");
    };
    assert_eq!(observed, machine);
    assert_eq!(node, node_id);
    collection.connected(&observed, node);
    let crate::Event::Fleet(observed) = receiving.recv().await.unwrap() else {
        panic!("the unchanged fleet must follow the lifecycle notice");
    };
    assert_eq!(observed.machines, fleet.machines);
    collection.answer(ticket, &machine, Ok(first));
    assert!(matches!(
        collection.rows().first().unwrap().progress,
        Progress::Failed(Problem::ConnectionChanged)
    ));
    assert!(!collection.report().unwrap().as_str().contains("Protocol:"));
    first_task.abort();
    second_task.abort();
}

async fn framed_client(local: DuplexStream) -> NodeClient {
    let (reader, writer) = tokio::io::split(local);
    NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(FRAME_BYTES).unwrap()),
        ClientInfo {
            name: "diagnostic-test".to_owned(),
            version: "0".to_owned(),
        },
        &ClientConfig::default(),
    )
    .await
    .unwrap()
}
fn fixture_state(
    events: tokio::sync::mpsc::Sender<crate::Event>,
) -> (crate::testing::Home, crate::State) {
    let home = crate::testing::Home::new();
    let (mut state, _receiving) = crate::testing::state(&home);
    state.events = events;
    (home, state)
}
fn admit(state: &mut crate::State, client: NodeClient, machine: &str) -> ConnectionContext {
    let binding = vessel_client::book::BookEntry {
        address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
        token: None,
        credential: None,
        id: None,
    };
    let connected = state
        .admit_connection(machine, &binding, client, Vec::new())
        .unwrap();
    let context = connected.context.clone();
    assert!(
        state
            .clients
            .insert(machine.to_owned(), connected)
            .is_none()
    );
    context
}
async fn collect_client(client: NodeClient) -> HostResult {
    let (events, _receiving) = tokio::sync::mpsc::channel(4);
    let (_home, mut state) = fixture_state(events);
    let context = admit(&mut state, client, "fixture");
    let selected = state.selected_connection(&context).unwrap();
    collect(&selected, &context).await.unwrap()
}
mod close;
mod held;
mod queue;
mod supported;

async fn welcome(remote: &mut DuplexStream, admitted: NodeId) {
    assert!(matches!(read(remote).await, ClientFrame::Hello(_)));
    write(
        remote,
        ServerFrame::Welcome(Welcome {
            protocol: vessel_proto::PROTOCOL_VERSION,
            node: node(admitted),
        }),
    )
    .await;
}
