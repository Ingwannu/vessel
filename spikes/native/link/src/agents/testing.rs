//! Finite method-specific peers; real admission and ordered read barriers.
//! Read and write refusal/wait modes remain separate so a wrong-method
//! mutation cannot satisfy a catalogue failure/deadline assertion.
use std::{
    future::{Future, poll_fn},
    num::NonZeroUsize,
    pin::Pin,
    time::Duration,
};
use tokio::sync::mpsc;
use tokio::task::{AbortHandle, JoinHandle};
use vessel_client::ClientConfig;
use vessel_client::NodeClient;
use vessel_proto::{Call, ErrorCode, ProtocolVersion};
use vessel_proto::{
    ClientFrame, ClientInfo, NodeInfo, NodeState, Outcome, ProtocolRange, Reply, ServerFrame,
    Welcome, WireError, agents::Catalogue,
};
use vessel_rpc::{FrameTransport, memory::MemoryTransport};

const PHASE: Duration = Duration::from_secs(5);
// One fixture safety window covers the longest intentional product advance
// (the45s catalogue read) plus setup and ordered barrier phases. A shorter
// idle timer would turn the timeout tests into false disconnect tests.
const OUTER: Duration = Duration::from_secs(75);
const CAPACITY: usize = 16;
pub(in crate::agents) const DIAGNOSTIC: &str = "registry fixture diagnostic";

pub(in crate::agents) async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    within(name, PHASE, future).await
}
pub(in crate::agents) async fn safety<T>(future: impl Future<Output = T>) -> T {
    within("registry test safety net", OUTER, future).await
}
async fn within<T>(name: &str, budget: Duration, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(budget, future)
        .await
        .unwrap_or_else(|_| panic!("{name} exceeded {budget:?}"))
}
pub(in crate::agents) struct Abort(AbortHandle);
impl Abort {
    pub(in crate::agents) fn of<T>(task: &JoinHandle<T>) -> Self {
        Self(task.abort_handle())
    }
}
impl Drop for Abort {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[derive(Clone, Copy)]
pub(in crate::agents) enum Mode {
    Catalogue,
    RefuseCatalogue(ErrorCode),
    WaitCatalogue,
    RefuseAdd(ErrorCode),
    WaitAdd,
    Add,
    WrongAdd,
}
pub(in crate::agents) struct Peer {
    pub(in crate::agents) client: NodeClient,
    pub(in crate::agents) seen: mpsc::Receiver<Call>,
    _abort: Abort,
}
async fn receive<T: FrameTransport>(transport: &mut T) -> Option<ClientFrame> {
    // This idle read spans intentional product time. Its budget cannot be
    // shorter than the test's advance or it would fake a disconnect.
    within(
        "registry peer next frame",
        OUTER,
        poll_fn(|cx| Pin::new(&mut *transport).poll_next(cx)),
    )
    .await
    .map(|raw| serde_json::from_str(&raw.unwrap()).unwrap())
}
async fn send<T: FrameTransport>(transport: &mut T, frame: ServerFrame) {
    phase(
        "registry peer writable",
        poll_fn(|cx| Pin::new(&mut *transport).poll_ready(cx)),
    )
    .await
    .unwrap();
    Pin::new(&mut *transport)
        .start_send(serde_json::to_string(&frame).unwrap())
        .unwrap();
    phase(
        "registry peer flush",
        poll_fn(|cx| Pin::new(&mut *transport).poll_flush(cx)),
    )
    .await
    .unwrap();
}
fn node() -> NodeInfo {
    NodeInfo {
        id: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        name: "registry-fixture".into(),
        hostname: "disposable".into(),
        os: "linux".into(),
        arch: "x86_64".into(),
        version: "0".into(),
        protocol: ProtocolRange::CURRENT,
        started_at: chrono::DateTime::UNIX_EPOCH,
        state: NodeState::Serving,
        public_key: None,
        timezone: None,
    }
}
async fn serve(
    mut remote: MemoryTransport,
    node: NodeInfo,
    protocol: ProtocolVersion,
    mode: Mode,
    notice: mpsc::Sender<Call>,
) {
    assert!(matches!(
        receive(&mut remote).await,
        Some(ClientFrame::Hello(_))
    ));
    send(
        &mut remote,
        ServerFrame::Welcome(Welcome {
            protocol,
            node: node.clone(),
        }),
    )
    .await;
    while let Some(frame) = receive(&mut remote).await {
        let ClientFrame::Request { id, call } = frame else {
            continue;
        };
        phase("registry peer observation", notice.send(call.clone()))
            .await
            .unwrap();
        // This fixture owns a method whitelist, not a second copy of the
        // entire root RPC table. Every unlisted call fails, including future
        // methods; a read-mode refusal can never accept a write frame.
        let outcome = if let Call::NodeInfo(_) = call {
            Some(Outcome::Ok(Reply::NodeInfo(node.clone())))
        } else if let Call::GetCatalogue(_) = call {
            match mode {
                Mode::Catalogue => Some(Outcome::Ok(Reply::GetCatalogue(Catalogue {
                    fetched_at: Some(chrono::DateTime::UNIX_EPOCH),
                    skipped: 1,
                    ..Catalogue::default()
                }))),
                Mode::RefuseCatalogue(code) => Some(Outcome::Err(WireError::new(code, DIAGNOSTIC))),
                Mode::WaitCatalogue => None,
                Mode::Add | Mode::WrongAdd | Mode::RefuseAdd(_) | Mode::WaitAdd => {
                    panic!("an addition fixture received a catalogue read")
                }
            }
        } else {
            let Call::AddAgent(call) = call else {
                panic!("unexpected registry fixture call: {call:?}")
            };
            match mode {
                Mode::Add | Mode::WrongAdd => {
                    use vessel_proto::agents::{
                        AddedAgent, AgentDetail, AgentInfo, AgentSource, AgentToAdd,
                    };
                    let source = match call.agent {
                        AgentToAdd::Command { .. } => AgentSource::Command,
                        AgentToAdd::Registry { id, version, .. } => {
                            AgentSource::Registry { id, version }
                        }
                    };
                    Some(Outcome::Ok(Reply::AddAgent(AgentInfo {
                        agent: vessel_proto::HarnessRef::Added(if matches!(mode, Mode::WrongAdd) {
                            "wrong-name".parse().unwrap()
                        } else {
                            call.name
                        }),
                        detail: AgentDetail::Added {
                            added: AddedAgent {
                                label: None,
                                program: "fixture-agent".parse().unwrap(),
                                args: Vec::new(),
                                env: Vec::new(),
                                secrets: Vec::new(),
                                inherit: Vec::new(),
                                mcp: Vec::new(),
                                source,
                            },
                        },
                    })))
                }
                Mode::RefuseAdd(code) => Some(Outcome::Err(WireError::new(code, DIAGNOSTIC))),
                Mode::WaitAdd => None,
                Mode::Catalogue | Mode::RefuseCatalogue(_) | Mode::WaitCatalogue => {
                    panic!("a catalogue fixture received an unauthorized mutation")
                }
            }
        };
        if let Some(outcome) = outcome {
            send(&mut remote, ServerFrame::Response { id, outcome }).await;
        }
    }
}
impl Peer {
    pub(in crate::agents) async fn new(protocol: ProtocolVersion, mode: Mode) -> Self {
        let (local, remote) = vessel_rpc::memory::pair(NonZeroUsize::new(CAPACITY).unwrap());

        let (notice, seen) = mpsc::channel(CAPACITY);
        let task = tokio::spawn(serve(remote, node(), protocol, mode, notice));
        // Retain cleanup before the first handshake await. Even a setup
        // failure or outer timeout cannot detach this peer indefinitely.
        let abort = Abort::of(&task);
        drop(task);
        let client = phase(
            "registry client handshake",
            NodeClient::over(
                local,
                ClientInfo {
                    name: "registry-tests".into(),
                    version: "0".into(),
                },
                &ClientConfig::default(),
            ),
        )
        .await
        .unwrap();
        Self {
            client,
            seen,
            _abort: abort,
        }
    }
    pub(in crate::agents) async fn barrier(&self) {
        let _node = phase(
            "registry ordered read barrier",
            self.client.call(vessel_proto::GetNode {}),
        )
        .await
        .unwrap();
    }
}
