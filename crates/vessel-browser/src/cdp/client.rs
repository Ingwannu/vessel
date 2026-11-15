//! One bounded multiplexed CDP connection; no actor/target command waits on another's result.

#[cfg(all(test, target_os = "linux"))]
mod candidate_tests;
#[cfg(test)]
mod pipe_tests;

use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use super::transport::{self, Transport};
use crate::delivery::{Attempt, Caller};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
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
pub(super) const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(6);

/// Errors remain typed; Debug never prints a frame, endpoint or page exception.
#[derive(thiserror::Error)]
pub(crate) enum CdpError {
    /// Connection establishment failed.
    #[error("CDP websocket operation failed")]
    Socket(#[source] tokio_tungstenite::tungstenite::Error),
    /// Child pipe failures never select a network fallback.
    #[error("CDP child pipe operation failed")]
    Pipe(#[source] std::io::Error),
    /// Malformed stdout retains only the content-free UTF-8 cause.
    #[error("CDP child pipe contains invalid UTF-8")]
    Utf8(#[source] std::str::Utf8Error),
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
    /// The same actual driver failure remains available after a cancelled waiter.
    #[error("CDP driver cleanup could not be confirmed")]
    Cleanup(#[source] Arc<CdpError>),
}
impl std::fmt::Debug for CdpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CdpError({self})")
    }
}

struct Command {
    id: CommandId,
    session: Option<SessionId>,
    text: String,
    attempt: Arc<Attempt>,
    answer: oneshot::Sender<Result<Payload, CdpError>>,
}
struct Queued {
    caller: Caller,
    result: oneshot::Receiver<Result<Payload, CdpError>>,
}
struct Pending {
    session: Option<SessionId>,
    answer: oneshot::Sender<Result<Payload, CdpError>>,
}
enum DriverState {
    Waiting(JoinHandle<Result<(), CdpError>>),
    Done(Result<(), Arc<CdpError>>),
}
struct Shared {
    commands: mpsc::Sender<Command>,
    next: AtomicU64,
    cancel: CancellationToken,
    driver: Mutex<DriverState>,
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
    pub(crate) async fn connect(endpoint: &Endpoint) -> Result<(Self, Events), CdpError> {
        let config = WebSocketConfig::default()
            .max_message_size(Some(wire::MAX_MESSAGE_BYTES))
            .max_frame_size(Some(wire::MAX_MESSAGE_BYTES));
        let (socket, _response) = tokio::time::timeout(
            CONNECT_TIMEOUT,
            tokio_tungstenite::connect_async_with_config(endpoint.as_str(), Some(config), false),
        )
        .await
        .map_err(|source| CdpError::TimedOut {
            effect: BrowserEffect::NotDelivered,
            source,
        })?
        .map_err(CdpError::Socket)?;
        Ok(Self::start(transport::WebSocket(socket)))
    }

    fn start<T: Transport>(socket: T) -> (Self, Events) {
        let (commands, receive) = mpsc::channel(COMMAND_CAPACITY);
        let (events, event_receive) = mpsc::channel(EVENT_CAPACITY);
        let cancel = CancellationToken::new();
        let driver = tokio::spawn(run(socket, receive, events, cancel.clone()));
        let shared = Arc::new(Shared {
            commands,
            next: AtomicU64::new(1),
            cancel,
            driver: Mutex::new(DriverState::Waiting(driver)),
        });
        (Self { shared }, Events(event_receive))
    }

    /// Send one sealed typed request to the owned target's flattened session.
    ///
    /// # Errors
    /// A timeout preserves delivery uncertainty; callers must not blindly retry mutations.
    pub(crate) async fn execute<R: Request>(
        &self,
        session: Option<SessionId>,
        request: R,
    ) -> Result<R::Response, CdpError> {
        self.execute_with_budget(session, request, COMMAND_TIMEOUT)
            .await
    }

    async fn execute_with_budget<R: Request>(
        &self,
        session: Option<SessionId>,
        request: R,
        budget: Duration,
    ) -> Result<R::Response, CdpError> {
        let Queued { caller, result } = self.enqueue(session, request)?;
        let outcome = tokio::time::timeout(budget, result)
            .await
            .map_err(|source| CdpError::TimedOut {
                effect: caller.cancel(),
                source,
            })?
            .map_err(|_closed| CdpError::Disconnected {
                effect: caller.cancel(),
            })?;
        let reply = match outcome {
            Err(CdpError::Closed) => {
                return Err(CdpError::Disconnected {
                    effect: caller.cancel(),
                });
            }
            result => result?,
        };
        reply.decode().map_err(CdpError::Json)
    }

    fn enqueue<R: Request>(
        &self,
        session: Option<SessionId>,
        request: R,
    ) -> Result<Queued, CdpError> {
        let sequence = self
            .shared
            .next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                next.checked_add(1)
            })
            .map_err(|_exhausted_sequence| CdpError::Closed)?;
        let id = CommandId::from_sequence(sequence).ok_or(CdpError::Closed)?;
        let outgoing = Outgoing {
            id,
            method: R::METHOD,
            params: Payload::encode(&request).map_err(CdpError::Json)?,
            session_id: session.clone(),
        };
        let text = serde_json::to_string(&outgoing).map_err(CdpError::Json)?;
        if text.len() > MAX_OUTGOING_BYTES {
            return Err(CdpError::Size);
        }
        let attempt = Arc::new(Attempt::new());
        let caller = Caller::new(Arc::clone(&attempt));
        let (answer, result) = oneshot::channel();
        self.shared
            .commands
            .try_send(Command {
                id,
                session,
                text,
                attempt,
                answer,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_command) => CdpError::Busy,
                mpsc::error::TrySendError::Closed(_command) => CdpError::Closed,
            })?;
        Ok(Queued { caller, result })
    }

    /// Retain the real driver join in the connection owner across cancelled waiters.
    /// Caller deadlines do not become cleanup receipts; after an abort request a
    /// later shutdown still joins the same task before confirming its outcome.
    pub(crate) async fn shutdown(&self) -> Result<(), CdpError> {
        self.shared.cancel.cancel();
        // Tokio lock intentionally spans the join: concurrent close callers must
        // neither remove this handle nor observe a fabricated empty success.
        let mut state = self.shared.driver.lock().await;
        let result = match &mut *state {
            DriverState::Done(result) => return result.clone().map_err(CdpError::Cleanup),
            DriverState::Waiting(driver) => {
                match tokio::time::timeout(SHUTDOWN_TIMEOUT, &mut *driver).await {
                    Ok(result) => result.map_err(CdpError::Task).and_then(|result| result),
                    Err(source) => {
                        // This task owns only this CDP socket/pending map. Keep
                        // its join even after asking it to abort; no process or
                        // profile cleanup proof follows from the abort request.
                        driver.abort();
                        return Err(CdpError::TimedOut {
                            effect: BrowserEffect::Uncertain,
                            source,
                        });
                    }
                }
            }
        }
        .map_err(Arc::new);
        *state = DriverState::Done(result.clone());
        result.map_err(CdpError::Cleanup)
    }

    /// Supervision can wait for loss without polling or inspecting raw CDP errors.
    pub(crate) async fn wait_closed(&self) {
        self.shared.commands.closed().await;
    }
}

async fn run<T: Transport>(
    mut socket: T,
    mut commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<Event>,
    cancel: CancellationToken,
) -> Result<(), CdpError> {
    let mut pending: BTreeMap<CommandId, Pending> = BTreeMap::new();
    let result = loop {
        pending.retain(|_, waiting| !waiting.answer.is_closed());
        tokio::select! {
            ()=cancel.cancelled()=>break Ok(()),
            command=commands.recv()=> {
                let Some(command)=command else {break Ok(())};
                if command.answer.is_closed() {continue;}
                if pending.len()>=PENDING_CAPACITY {let _gone=command.answer.send(Err(CdpError::Busy));continue;}
                // Withdrawal and the start of any write have one atomic winner.
                // A caller told NotDelivered can no longer become a late send.
                if !command.attempt.start() {continue;}
                let _old=pending.insert(command.id,Pending{session:command.session,answer:command.answer});
                tokio::select! {
                    biased;
                    ()=cancel.cancelled()=>break Ok(()),
                    sent=tokio::time::timeout(WRITE_TIMEOUT,socket.send(command.text))=>match sent {
                        Ok(Ok(()))=>{},Ok(Err(error))=>break Err(error),Err(source)=>break Err(CdpError::TimedOut{effect:BrowserEffect::Uncertain,source}),
                    }
                }
            },
            message=socket.next()=> {
                let text=match message {
                    Ok(Some(text))=>text,
                    Ok(None)=>break Ok(()),
                    Err(error)=>break Err(error),
                };
                match wire::incoming(&text) {
                    Ok(Incoming::Response{id,session,result})=>if let Some(waiting)=pending.remove(&id) {
                        // Command ids alone do not prove which flattened target answered.
                        if waiting.session != session {
                            let _gone=waiting.answer.send(Err(CdpError::Protocol));
                            break Err(CdpError::Protocol);
                        }
                        let _gone=waiting.answer.send(result.map_err(CdpError::Remote));
                    },
                    Ok(Incoming::Event(event))=>if events.try_send(event).is_err() {break Err(CdpError::Busy);},
                    Ok(Incoming::UnknownEvent)=>{},
                    Err(error)=>break Err(CdpError::Decode(error)),
                }
            },
        }
    };
    for waiting in pending.into_values() {
        let _gone = waiting.answer.send(Err(CdpError::Closed));
    }
    // Dropping this owned pipe/socket is the hard close; a blocked peer cannot
    // make shutdown wait for another application-level closing handshake.
    drop(socket);
    result
}
