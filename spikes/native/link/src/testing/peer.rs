//! The ONE typed framing edge for isolated actor regressions and App scenarios.
//! Wire bytes are bounded before allocation/serialization; raw decode text is
//! discarded deliberately, so a synthetic credential cannot enter an error.
use super::{FixtureError, Phase, failure};
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _},
    sync::mpsc,
    task::JoinHandle,
};
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Ack, Call, ClientFrame, ClientInfo, ErrorCode, NodeId, NodeInfo, NodeState, Outcome,
    ProtocolRange, Reply, ServerFrame, SessionId, SessionInfo, SessionMode, SessionSpec,
    Subscribed, SubscriptionId, Welcome, WireError,
};
mod encode;
#[cfg(test)]
mod expectations;
#[cfg(test)]
pub(crate) use expectations::{connect, controlled};

const FRAME_BYTES: usize = 65_536;
const CHANNEL_CAPACITY: usize = 4;
const FAILURE_CAPACITY: usize = 1;
const MAX_RECORDED_CALLS: usize = 64;

pub(crate) struct Peer {
    pub client: Option<NodeClient>,
    pub calls: Arc<Mutex<Vec<Call>>>,
    pub subscribed: mpsc::Receiver<()>,
    pub task: Option<JoinHandle<()>>,
    pub requests: Option<mpsc::Receiver<Request>>,
    frames: mpsc::Sender<Frame>,
    failures: mpsc::Receiver<FixtureError>,
}
pub(crate) struct Request {
    pub call: Call,
    pub answer: tokio::sync::oneshot::Sender<Outcome>,
}
struct Frame {
    frame: ServerFrame,
    sent: tokio::sync::oneshot::Sender<bool>,
}

async fn read(stream: &mut (impl AsyncRead + Unpin)) -> Result<Option<ClientFrame>, FixtureError> {
    let length = match stream.read_u32().await {
        Ok(length) => length,
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(source) => {
            return Err(FixtureError::Io {
                phase: Phase::Read,
                source,
            });
        }
    };
    let length = usize::try_from(length).map_err(|_range| FixtureError::Protocol)?;
    if length > FRAME_BYTES {
        return Err(FixtureError::Protocol);
    }
    let mut bytes = vec![0; length];
    let _read = stream
        .read_exact(&mut bytes)
        .await
        .map_err(|source| FixtureError::Io {
            phase: Phase::Read,
            source,
        })?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_raw_decode| FixtureError::Protocol)
}
async fn write(
    stream: &mut (impl AsyncWrite + Unpin),
    frame: ServerFrame,
) -> Result<bool, FixtureError> {
    let bytes = encode::frame(&frame)?;
    let length = u32::try_from(bytes.len()).map_err(|_range| FixtureError::Protocol)?;
    if stream.write_u32(length).await.is_err() {
        return Ok(false);
    }
    Ok(stream.write_all(&bytes).await.is_ok())
}
async fn send(frames: &mpsc::Sender<Frame>, frame: ServerFrame) -> Result<bool, FixtureError> {
    let (sent, receipt) = tokio::sync::oneshot::channel();
    if frames.send(Frame { frame, sent }).await.is_err() {
        return Ok(false);
    }
    failure::phase(Phase::Write, receipt)
        .await?
        .map_err(|_closed| FixtureError::Closed(Phase::Write))
}

pub(crate) fn info(session: SessionId) -> SessionInfo {
    SessionInfo::new(
        session,
        SessionSpec::new(
            vessel_proto::HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    )
}
fn node(name: &str, id: NodeId) -> NodeInfo {
    NodeInfo {
        id,
        name: name.to_owned(),
        hostname: "test-machine".to_owned(),
        os: "linux".to_owned(),
        arch: "x86_64".to_owned(),
        version: "0.1.0".to_owned(),
        protocol: ProtocolRange::CURRENT,
        started_at: chrono::DateTime::UNIX_EPOCH,
        state: NodeState::Serving,
        public_key: None,
        timezone: None,
    }
}

pub(super) async fn try_controlled(name: &str, id: NodeId) -> Result<Peer, FixtureError> {
    let (send, receive) = mpsc::channel(CHANNEL_CAPACITY);
    let mut peer = start(name, id, Some(send)).await?;
    peer.requests = Some(receive);
    Ok(peer)
}
async fn start(
    name: &str,
    id: NodeId,
    control: Option<mpsc::Sender<Request>>,
) -> Result<Peer, FixtureError> {
    let (local, remote) = tokio::io::duplex(FRAME_BYTES);
    let (remote, writer) = tokio::io::split(remote);
    let (frames, outgoing) = mpsc::channel::<Frame>(CHANNEL_CAPACITY);
    let (ready, subscribed) = mpsc::channel(CHANNEL_CAPACITY);
    let (errors, failures) = mpsc::channel(FAILURE_CAPACITY);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let recording = Arc::clone(&calls);
    let replies = frames.clone();
    let node = node(name, id);
    let task = tokio::spawn(async move {
        let reading = reading(remote, replies, recording, ready, node, control);
        let writing = writing(writer, outgoing);
        let result = tokio::select! { result = reading => result, result = writing => result };
        if let Err(error) = result {
            let _closed = errors.try_send(error);
        }
    });
    // Cleanup owns the task before the first awaited handshake, even on error.
    let mut peer = Peer {
        client: None,
        calls,
        subscribed,
        task: Some(task),
        requests: None,
        frames,
        failures,
    };
    let (reader, writer) = tokio::io::split(local);
    let bound = NonZeroUsize::new(FRAME_BYTES).ok_or(FixtureError::Protocol)?;
    let client = failure::phase(
        Phase::Setup,
        NodeClient::over(
            vessel_rpc::Framed::new(reader, writer, bound),
            ClientInfo {
                name: "machine-book-tests".to_owned(),
                version: "0".to_owned(),
            },
            &ClientConfig::default(),
        ),
    )
    .await?
    .map_err(|_raw_peer_failure| FixtureError::Protocol)?;
    peer.client = Some(client);
    Ok(peer)
}

async fn reading(
    mut remote: impl AsyncRead + Unpin,
    replies: mpsc::Sender<Frame>,
    recording: Arc<Mutex<Vec<Call>>>,
    ready: mpsc::Sender<()>,
    node: NodeInfo,
    control: Option<mpsc::Sender<Request>>,
) -> Result<(), FixtureError> {
    if !matches!(read(&mut remote).await?, Some(ClientFrame::Hello(_))) {
        return Err(FixtureError::Protocol);
    }
    if !send(
        &replies,
        ServerFrame::Welcome(Welcome {
            protocol: vessel_proto::PROTOCOL_VERSION,
            node,
        }),
    )
    .await?
    {
        return Ok(());
    }
    while let Some(frame) = read(&mut remote).await? {
        if let ClientFrame::Request { id, call } = frame {
            {
                let mut calls = recording.lock().map_err(|_poison| FixtureError::Worker)?;
                if calls.len() >= MAX_RECORDED_CALLS {
                    return Err(FixtureError::Full);
                }
                calls.push(call.clone());
            }
            let reply = response(call, control.as_ref(), &ready).await?;
            if !send(&replies, ServerFrame::Response { id, outcome: reply }).await? {
                break;
            }
        }
    }
    Ok(())
}
async fn response(
    call: Call,
    control: Option<&mpsc::Sender<Request>>,
    ready: &mpsc::Sender<()>,
) -> Result<Outcome, FixtureError> {
    if let Some(control) = control {
        let (answer, receive) = tokio::sync::oneshot::channel();
        control
            .send(Request { call, answer })
            .await
            .map_err(|_closed| FixtureError::Closed(Phase::Read))?;
        return receive
            .await
            .map_err(|_closed| FixtureError::Closed(Phase::Read));
    }
    Ok(if matches!(&call, Call::ListSessions(_)) {
        Outcome::Ok(Reply::ListSessions(Vec::new()))
    } else if let Call::Subscribe(ask) = call {
        ready
            .send(())
            .await
            .map_err(|_closed| FixtureError::Closed(Phase::Read))?;
        Outcome::Ok(Reply::Subscribe(Subscribed {
            subscription: SubscriptionId::new(1),
            info: info(ask.session),
        }))
    } else if matches!(&call, Call::Unsubscribe(_)) {
        Outcome::Ok(Reply::Unsubscribe(Ack {}))
    } else {
        Outcome::Err(WireError::new(
            ErrorCode::Unsupported,
            "not used by machine-book tests",
        ))
    })
}
async fn writing(
    mut writer: impl AsyncWrite + Unpin,
    mut outgoing: mpsc::Receiver<Frame>,
) -> Result<(), FixtureError> {
    while let Some(Frame { frame, sent }) = outgoing.recv().await {
        let success = write(&mut writer, frame).await?;
        let _receipt = sent.send(success);
        if !success {
            break;
        }
    }
    Ok(())
}
impl Peer {
    pub(super) async fn close(&mut self) -> Result<(), FixtureError> {
        self.subscribed.close();
        if let Some(task) = self.task.as_mut() {
            task.abort();
            if let Err(error) = failure::phase(Phase::Close, task).await?
                && !error.is_cancelled()
            {
                return Err(FixtureError::Worker);
            }
        }
        let _joined = self.task.take();
        failure::phase(Phase::Close, self.frames.closed()).await?;
        match self.failures.try_recv() {
            Ok(FixtureError::Closed(Phase::Read | Phase::Write))
            | Err(mpsc::error::TryRecvError::Empty | mpsc::error::TryRecvError::Disconnected) => {
                Ok(())
            }
            Ok(error) => Err(error),
        }
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
