//! One bounded multiplexed CDP connection; no actor/target command waits on another's result.

use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::{Message, protocol::WebSocketConfig};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tokio_util::sync::CancellationToken;
use vessel_proto::browser::{BrowserEffect, COMMAND_TIMEOUT};

use super::Endpoint;
use super::identity::{CommandId, SessionId};
use super::requests::Request;
use super::wire::{self, Event, Incoming, Outgoing, Payload};

const COMMAND_CAPACITY: usize = 64;
const PENDING_CAPACITY: usize = 16;
const EVENT_CAPACITY: usize = 32;
const MAX_OUTGOING_BYTES: usize = 256 * 1024;
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(6);
type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Errors remain typed; Debug never prints a frame, endpoint or page exception.
#[derive(thiserror::Error)]
pub(crate) enum Error {
    /// Connection establishment failed.
    #[error("CDP websocket operation failed")]
    Socket(#[source] tokio_tungstenite::tungstenite::Error),
    /// A foreign response could not be interpreted.
    #[error("CDP message could not be decoded")]
    Decode(#[source] wire::DecodeError),
    /// Local typed parameters could not be encoded.
    #[error("CDP command encoding failed")]
    Json(#[source] serde_json::Error),
    /// A known operation was refused by Chromium; its text is not a node diagnostic.
    #[error("CDP operation was refused by the browser")]
    Remote(#[source] wire::RemoteError),
    /// Refuse rather than grow a command/pending/event queue.
    #[error("CDP connection capacity is exhausted")]
    Busy,
    /// A frame/control command does not fit its bound.
    #[error("CDP command exceeds its size limit")]
    Size,
    /// Losing the driver cannot fabricate a successful operation.
    #[error("CDP connection closed")]
    Closed,
    /// A lost connection after delivery may already have changed the page.
    #[error("CDP response was lost ({effect:?})")]
    Disconnected {
        /// A caller must not infer permission to replay from connection loss.
        effect: BrowserEffect,
    },
    /// Unexpected binary websocket data is not a JSON response.
    #[error("CDP peer sent a non-JSON message")]
    Protocol,
    /// Delivery is marked before socket send can have a partial side effect.
    #[error("CDP command timed out ({effect:?})")]
    TimedOut {
        /// The actor uses this fact to prevent automatic mutation replay.
        effect: BrowserEffect,
        /// Keep the timer cause without its selected URL/function.
        #[source]
        source: tokio::time::error::Elapsed,
    },
    /// Actor failure is not swallowed by shutdown.
    #[error("CDP driver task failed")]
    Task(#[source] tokio::task::JoinError),
}
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CdpError({self})")
    }
}

struct Command {
    id: CommandId,
    text: String,
    delivered: Arc<AtomicBool>,
    answer: oneshot::Sender<Result<Payload, Error>>,
}
struct Pending {
    answer: oneshot::Sender<Result<Payload, Error>>,
}
struct Shared {
    commands: mpsc::Sender<Command>,
    next: AtomicU64,
    cancel: CancellationToken,
    driver: Mutex<Option<JoinHandle<Result<(), Error>>>>,
}
impl Drop for Shared {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// A trusted engine's private debugger connection, never an application RPC surface.
#[derive(Clone)]
pub(crate) struct Client {
    shared: Arc<Shared>,
}
impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CdpClient").finish_non_exhaustive()
    }
}
/// Essential state signals; overflow fails the engine instead of losing invalidation/dialog events.
pub(crate) struct Events(mpsc::Receiver<Event>);
impl std::fmt::Debug for Events {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CdpEvents").finish_non_exhaustive()
    }
}
impl Events {
    /// None means the engine connection ended; no fabricated final descriptor follows.
    pub(crate) async fn next(&mut self) -> Option<Event> {
        self.0.recv().await
    }
}

impl Client {
    /// Connect only after the launcher independently verifies child/endpoint ownership.
    ///
    /// # Errors
    /// Socket/connect deadline failures refuse; no runtime is qualified here.
    pub(crate) async fn connect(endpoint: &Endpoint) -> Result<(Self, Events), Error> {
        let config = WebSocketConfig::default()
            .max_message_size(Some(wire::MAX_MESSAGE_BYTES))
            .max_frame_size(Some(wire::MAX_MESSAGE_BYTES));
        let (socket, _response) = tokio::time::timeout(
            CONNECT_TIMEOUT,
            tokio_tungstenite::connect_async_with_config(endpoint.as_str(), Some(config), false),
        )
        .await
        .map_err(|source| Error::TimedOut {
            effect: BrowserEffect::NotDelivered,
            source,
        })?
        .map_err(Error::Socket)?;
        let (commands, receive) = mpsc::channel(COMMAND_CAPACITY);
        let (events, event_receive) = mpsc::channel(EVENT_CAPACITY);
        let cancel = CancellationToken::new();
        let driver = tokio::spawn(run(socket, receive, events, cancel.clone()));
        let shared = Arc::new(Shared {
            commands,
            next: AtomicU64::new(1),
            cancel,
            driver: Mutex::new(Some(driver)),
        });
        Ok((Self { shared }, Events(event_receive)))
    }

    /// Send one sealed typed request to the owned target's flattened session.
    ///
    /// # Errors
    /// A timeout preserves delivery uncertainty; callers must not blindly retry mutations.
    pub(crate) async fn execute<R: Request>(
        &self,
        session: Option<SessionId>,
        request: R,
    ) -> Result<R::Response, Error> {
        let sequence = self
            .shared
            .next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                next.checked_add(1)
            })
            .map_err(|_| Error::Closed)?;
        let id = CommandId::from_sequence(sequence).ok_or(Error::Closed)?;
        let outgoing = Outgoing {
            id,
            method: R::METHOD,
            params: Payload::encode(&request).map_err(Error::Json)?,
            session_id: session,
        };
        let text = serde_json::to_string(&outgoing).map_err(Error::Json)?;
        if text.len() > MAX_OUTGOING_BYTES {
            return Err(Error::Size);
        }
        let delivered = Arc::new(AtomicBool::new(false));
        let (answer, result) = oneshot::channel();
        self.shared
            .commands
            .try_send(Command {
                id,
                text,
                delivered: Arc::clone(&delivered),
                answer,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_command) => Error::Busy,
                mpsc::error::TrySendError::Closed(_command) => Error::Closed,
            })?;
        let outcome = tokio::time::timeout(COMMAND_TIMEOUT, result)
            .await
            .map_err(|source| Error::TimedOut {
                effect: delivery(&delivered),
                source,
            })?
            .map_err(|_closed| Error::Disconnected {
                effect: delivery(&delivered),
            })?;
        let reply = match outcome {
            Err(Error::Closed) => {
                return Err(Error::Disconnected {
                    effect: delivery(&delivered),
                });
            }
            result => result?,
        };
        reply.decode().map_err(Error::Json)
    }

    /// Close the connection before profile cleanup; joining retains the driver's original error.
    ///
    /// # Errors
    /// The engine sees socket/protocol/task failure, never an invented successful close.
    pub(crate) async fn shutdown(self) -> Result<(), Error> {
        self.shared.cancel.cancel();
        let driver = self.shared.driver.lock().await.take();
        match driver {
            Some(mut driver) => match tokio::time::timeout(SHUTDOWN_TIMEOUT, &mut driver).await {
                Ok(result) => result.map_err(Error::Task)?,
                Err(source) => {
                    // This task owns only this CDP socket/pending map. Aborting
                    // it cannot signal any unrelated process or connection.
                    driver.abort();
                    let _stopped = driver.await;
                    Err(Error::TimedOut {
                        effect: BrowserEffect::Uncertain,
                        source,
                    })
                }
            },
            None => Ok(()),
        }
    }

    /// Supervision can wait for loss without polling or inspecting raw CDP errors.
    pub(crate) async fn wait_closed(&self) {
        self.shared.commands.closed().await;
    }
}

async fn run(
    mut socket: Socket,
    mut commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<Event>,
    cancel: CancellationToken,
) -> Result<(), Error> {
    let mut pending: BTreeMap<CommandId, Pending> = BTreeMap::new();
    let result = loop {
        pending.retain(|_, waiting| !waiting.answer.is_closed());
        tokio::select! {
            ()=cancel.cancelled()=>break Ok(()),
            command=commands.recv()=> {
                let Some(command)=command else {break Ok(())};
                if command.answer.is_closed() {continue;}
                if pending.len()>=PENDING_CAPACITY {let _gone=command.answer.send(Err(Error::Busy));continue;}
                let _old=pending.insert(command.id,Pending{answer:command.answer});
                command.delivered.store(true,Ordering::Release);
                match tokio::time::timeout(WRITE_TIMEOUT,socket.send(Message::Text(command.text.into()))).await {
                    Ok(Ok(()))=>{},Ok(Err(error))=>break Err(Error::Socket(error)),Err(source)=>break Err(Error::TimedOut{effect:BrowserEffect::Uncertain,source}),
                }
            },
            message=socket.next()=> {
                let text=match message {
                    Some(Ok(Message::Text(text)))=>text,
                    Some(Ok(Message::Ping(_)))=>{
                        match tokio::time::timeout(WRITE_TIMEOUT,socket.flush()).await {
                            Ok(Ok(()))=>continue,
                            Ok(Err(error))=>break Err(Error::Socket(error)),
                            Err(source)=>break Err(Error::TimedOut{effect:BrowserEffect::NotDelivered,source}),
                        }
                    },
                    Some(Ok(Message::Pong(_)))=>continue,
                    Some(Ok(Message::Close(_)))|None=>break Ok(()),
                    Some(Ok(Message::Binary(_)|Message::Frame(_)))=>break Err(Error::Protocol),
                    Some(Err(error))=>break Err(Error::Socket(error)),
                };
                match wire::incoming(&text) {
                    Ok(Incoming::Response{id,result})=>if let Some(waiting)=pending.remove(&id) {let _gone=waiting.answer.send(result.map_err(Error::Remote));},
                    Ok(Incoming::Event(event))=>if events.try_send(event).is_err() {break Err(Error::Busy);},
                    Ok(Incoming::UnknownEvent)=>{},
                    Err(error)=>break Err(Error::Decode(error)),
                }
            },
        }
    };
    for waiting in pending.into_values() {
        let _gone = waiting.answer.send(Err(Error::Closed));
    }
    // Dropping this owned TCP stream is the hard close; a blocked peer cannot
    // make shutdown wait for another application-level closing handshake.
    drop(socket);
    result
}

fn delivery(delivered: &AtomicBool) -> BrowserEffect {
    if delivered.load(Ordering::Acquire) {
        BrowserEffect::Uncertain
    } else {
        BrowserEffect::NotDelivered
    }
}
