//! Two real client subscriptions over in-memory framed RPC; no socket or agent.
use super::*;
use std::{collections::HashMap, num::NonZeroUsize, time::Duration};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use vessel_app_view::{devices::MachineKey, side_question::Target};
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Ack, AgentEvent, Call, ClientFrame, ClientInfo, ContentBlock, EventRecord, NodeId, NodeInfo,
    NodeState, Outcome, ProtocolRange, Reply, RequestId, ServerFrame, SessionEvent, SessionId,
    SessionInfo, SessionMode, SessionSpec, Subscribed, SubscriptionId, Welcome,
};

const FRAME: usize = 64 * 1024;
const ROOM: usize = 8;
const DEADLINE: Duration = Duration::from_secs(3);

async fn read<R: AsyncRead + Unpin>(stream: &mut R) -> Option<ClientFrame> {
    let count = stream.read_u32().await.ok()?;
    let count = usize::try_from(count).unwrap();
    assert!(count <= FRAME);
    let mut bytes = vec![0; count];
    stream.read_exact(&mut bytes).await.ok()?;
    Some(serde_json::from_slice(&bytes).unwrap())
}
async fn write<W: AsyncWrite + Unpin>(stream: &mut W, frame: ServerFrame) -> bool {
    let bytes = serde_json::to_vec(&frame).unwrap();
    if stream
        .write_u32(u32::try_from(bytes.len()).unwrap())
        .await
        .is_err()
    {
        return false;
    }
    stream.write_all(&bytes).await.is_ok()
}
fn info(session: SessionId) -> SessionInfo {
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
struct Peer {
    client: NodeClient,
    emit: mpsc::Sender<SessionId>,
    subscribed: mpsc::Receiver<SessionId>,
    task: JoinHandle<()>,
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct Abort(JoinHandle<()>);
impl Drop for Abort {
    fn drop(&mut self) {
        self.0.abort();
    }
}

enum Incoming {
    Frame(Option<ClientFrame>),
    Emit(Option<SessionId>),
}

async fn peer() -> Peer {
    let (local, mut remote) = tokio::io::duplex(FRAME);
    let (emit, mut emissions) = mpsc::channel(ROOM);
    let (ready, subscribed) = mpsc::channel(ROOM);
    let task = tokio::spawn(async move {
        assert!(matches!(
            read(&mut remote).await,
            Some(ClientFrame::Hello(_))
        ));
        assert!(
            write(
                &mut remote,
                ServerFrame::Welcome(Welcome {
                    protocol: vessel_proto::PROTOCOL_VERSION,
                    node: NodeInfo {
                        id: NodeId::random(),
                        name: "fake".to_owned(),
                        hostname: "fake".to_owned(),
                        os: "linux".to_owned(),
                        arch: "test".to_owned(),
                        version: "0".to_owned(),
                        protocol: ProtocolRange::CURRENT,
                        started_at: chrono::DateTime::UNIX_EPOCH,
                        state: NodeState::Serving,
                        public_key: None,
                        timezone: None
                    }
                })
            )
            .await
        );
        let (mut reader, mut writer) = tokio::io::split(remote);
        let (frames, mut incoming_frames) = mpsc::channel(ROOM);
        let _reader = Abort(tokio::spawn(async move {
            while let Some(frame) = read(&mut reader).await {
                if frames.send(frame).await.is_err() {
                    break;
                }
            }
        }));
        let mut subscriptions = HashMap::new();
        let mut sequence = HashMap::new();
        let mut next = 0_u64;
        loop {
            let incoming = tokio::select! { frame = incoming_frames.recv() => Incoming::Frame(frame), session = emissions.recv() => Incoming::Emit(session) };
            match incoming {
                Incoming::Frame(Some(ClientFrame::Request { id, call })) => {
                    let reply = if let Call::Subscribe(ask) = call {
                        next = next.checked_add(1).unwrap();
                        let subscription = SubscriptionId::new(next);
                        let _previous = subscriptions.insert(ask.session, subscription);
                        ready.send(ask.session).await.unwrap();
                        Reply::Subscribe(Subscribed {
                            subscription,
                            info: info(ask.session),
                        })
                    } else if let Call::Unsubscribe(_) = call {
                        Reply::Unsubscribe(Ack {})
                    } else {
                        panic!("unexpected fake call");
                    };
                    if !write(
                        &mut writer,
                        ServerFrame::Response {
                            id,
                            outcome: Outcome::Ok(reply),
                        },
                    )
                    .await
                    {
                        break;
                    }
                }
                Incoming::Frame(Some(ClientFrame::Cancel { .. })) => {}
                Incoming::Frame(Some(ClientFrame::Hello(_))) => panic!("second hello"),
                Incoming::Frame(None) | Incoming::Emit(None) => break,
                Incoming::Emit(Some(session)) => {
                    let subscription = *subscriptions.get(&session).unwrap();
                    let seq = sequence.entry(session).or_insert(0_u64);
                    let record = EventRecord {
                        seq: EventSeq::new(*seq),
                        at: chrono::DateTime::UNIX_EPOCH,
                        event: SessionEvent::Agent {
                            event: AgentEvent::MessageChunk {
                                content: ContentBlock::Text {
                                    text: "reply".to_owned(),
                                },
                            },
                        },
                    };
                    *seq = seq.checked_add(1).unwrap();
                    if !write(
                        &mut writer,
                        ServerFrame::Event {
                            subscription,
                            record,
                        },
                    )
                    .await
                    {
                        break;
                    }
                }
            }
        }
    });
    let (reader, writer) = tokio::io::split(local);
    let client = NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(FRAME).unwrap()),
        ClientInfo {
            name: "side-tests".to_owned(),
            version: "0".to_owned(),
        },
        &ClientConfig::default(),
    )
    .await
    .unwrap();
    Peer {
        client,
        emit,
        subscribed,
        task,
    }
}
async fn receive<T>(receiver: &mut mpsc::Receiver<T>) -> T {
    tokio::time::timeout(DEADLINE, receiver.recv())
        .await
        .expect("in-memory peer deadline")
        .expect("channel stays open")
}

#[tokio::test]
async fn closing_side_subscription_keeps_main_follow_alive_and_events_separate() {
    let mut peer = peer().await;
    let parent = SessionId::random();
    let side = SessionId::random();
    let follow = Follow {
        target: Target {
            machine: MachineKey::from_book("fake"),
            parent,
        },
        side,
        ticket: RequestId::new(1),
    };
    let (events, mut inbox) = mpsc::channel(ROOM);
    let main = tokio::spawn(crate::follow(peer.client.clone(), parent, events.clone()));
    assert_eq!(receive(&mut peer.subscribed).await, parent);
    let mut stream = Stream::default();
    stream.watch(follow.clone(), Some(peer.client.clone()), events);
    assert_eq!(receive(&mut peer.subscribed).await, side);
    peer.emit.send(side).await.unwrap();
    let crate::Event::SideQuestion(Event::Records {
        follow: got,
        records,
    }) = receive(&mut inbox).await
    else {
        panic!("side event")
    };
    assert_eq!(got, follow);
    assert_eq!(records.len(), 1);
    stream.close();
    peer.emit.send(parent).await.unwrap();
    let crate::Event::Records { session, records } = receive(&mut inbox).await else {
        panic!("main event")
    };
    assert_eq!(session, parent);
    assert_eq!(records.len(), 1);
    assert!(!main.is_finished());
    main.abort();
    assert!(main.await.unwrap_err().is_cancelled());
}
