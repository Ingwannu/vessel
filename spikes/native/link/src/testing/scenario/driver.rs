//! Single fixture owner: all controls and App commands use the existing State.
use super::{
    Control, FixtureError, Home, Lifetime, Phase, Port, Reconnect, Response, failure, peer,
};
use crate::{Command, Event, State};
use tokio::sync::{mpsc, oneshot, watch};
use vessel_app_view::connection::ConnectionContext;
use vessel_proto::{NodeId, Outcome};

pub(super) struct Seed {
    pub state: State,
    pub peer: peer::Peer,
    pub home: Home,
    pub context: ConnectionContext,
}
pub(super) struct Channels {
    pub commands: mpsc::Receiver<Command>,
    pub events: mpsc::Receiver<Event>,
    pub controls: mpsc::Receiver<Control>,
    pub stopped: watch::Receiver<Lifetime>,
}
pub(super) struct Driver {
    // Drop order invalidates admission before aborting the peer or removing home.
    state: Option<State>,
    peer: peer::Peer,
    home: Home,
    context: ConnectionContext,
    channels: Channels,
    request_wait: Option<Port>,
    event_wait: Option<Port>,
    pending: Option<oneshot::Sender<Outcome>>,
    stop_ack: Option<Port>,
}
impl Driver {
    pub(super) fn new(seed: Seed, channels: Channels) -> Self {
        Self {
            state: Some(seed.state),
            peer: seed.peer,
            home: seed.home,
            context: seed.context,
            channels,
            request_wait: None,
            event_wait: None,
            pending: None,
            stop_ack: None,
        }
    }
    pub(super) async fn run(mut self) -> Result<(), FixtureError> {
        let result = Box::pin(self.drive()).await;
        let cleanup = self.cleanup().await;
        match result.and(cleanup) {
            Ok(()) => {
                if let Some(port) = self.stop_ack.take() {
                    answer(port, Ok(Response::Done));
                }
                Ok(())
            }
            Err(error) => {
                if let Some(port) = self.stop_ack.take() {
                    answer(port, Err(error));
                    Err(FixtureError::Worker)
                } else {
                    Err(error)
                }
            }
        }
    }
    async fn drive(&mut self) -> Result<(), FixtureError> {
        loop {
            prune(&mut self.request_wait);
            prune(&mut self.event_wait);
            let wants_call = self.request_wait.is_some() && self.pending.is_none();
            let wants_event = self.event_wait.is_some();
            tokio::select! {
                biased;
                _stopped = self.channels.stopped.changed() => return Ok(()),
                command = self.channels.commands.recv() => {
                    let Some(command) = command else { return Ok(()); };
                    Box::pin(self.dispatch(command)).await?;
                },
                control = self.channels.controls.recv() => {
                    let Some(control) = control else { return Ok(()); };
                    if Box::pin(self.control(control)).await? { return Ok(()); }
                },
                call = async {
                    match self.peer.requests.as_mut() {
                        Some(requests) => requests.recv().await,
                        None => None,
                    }
                }, if wants_call => self.received_call(call),
                event = self.channels.events.recv(), if wants_event => {
                    if let Some(port) = self.event_wait.take() {
                        answer(port, event.map(|event| Response::Event(Box::new(event)))
                            .ok_or(FixtureError::Closed(Phase::Control)));
                    }
                },
            }
        }
    }
    fn received_call(&mut self, call: Option<peer::Request>) {
        if let Some(port) = self.request_wait.take() {
            match call {
                Some(peer::Request {
                    call,
                    answer: reply,
                }) => {
                    self.pending = Some(reply);
                    answer(port, Ok(Response::Call(Box::new(call))));
                }
                None => answer(port, Err(FixtureError::Closed(Phase::Read))),
            }
        }
    }
    async fn dispatch(&mut self, command: Command) -> Result<(), FixtureError> {
        let state = self
            .state
            .as_mut()
            .ok_or(FixtureError::Closed(Phase::Dispatch))?;
        failure::phase(Phase::Dispatch, state.handle(command)).await
    }
    async fn control(&mut self, control: Control) -> Result<bool, FixtureError> {
        match control {
            Control::Request(port) => {
                if self.request_wait.is_some() || self.pending.is_some() {
                    answer(port, Err(FixtureError::Full));
                } else {
                    self.request_wait = Some(port);
                }
            }
            Control::Event(port) => {
                if self.event_wait.is_some() {
                    answer(port, Err(FixtureError::Full));
                } else {
                    self.event_wait = Some(port);
                }
            }
            Control::Reply(outcome, port) => {
                let result = self
                    .pending
                    .take()
                    .ok_or(FixtureError::NoPendingCall)
                    .and_then(|pending| {
                        pending
                            .send(*outcome)
                            .map_err(|_discarded| FixtureError::Closed(Phase::Read))
                    })
                    .map(|()| Response::Done);
                answer(port, result);
            }
            Control::Retire(port) => {
                let result = Box::pin(self.dispatch(Command::RetireConnection(self.context.tag())))
                    .await
                    .map(|()| Response::Done);
                answer(port, result);
            }
            Control::Reconnect(change, port) => {
                let result = Box::pin(self.reconnect(change))
                    .await
                    .map(Response::Context);
                answer(port, result);
            }
            Control::ClosePeer(port) => {
                answer(port, self.peer.close().await.map(|()| Response::Done));
            }
            Control::Transcript(port) => {
                let calls = self
                    .peer
                    .calls
                    .lock()
                    .map_err(|_poison| FixtureError::Worker)?
                    .clone();
                answer(port, Ok(Response::Transcript(calls)));
            }
            Control::Stop(port) => {
                self.stop_ack = Some(port);
                return Ok(true);
            }
        }
        Ok(false)
    }
    async fn reconnect(&mut self, change: Reconnect) -> Result<ConnectionContext, FixtureError> {
        // Irreversible original retirement precedes every new handshake await.
        Box::pin(self.dispatch(Command::RetireConnection(self.context.tag()))).await?;
        drop(self.pending.take());
        self.peer.close().await?;
        let id = match change {
            Reconnect::SameNode => self.context.node(),
            Reconnect::DifferentNode => NodeId::random(),
        };
        let name = self.context.machine().as_str();
        let mut next = peer::try_controlled(name, id).await?;
        let state = self
            .state
            .as_mut()
            .ok_or(FixtureError::Closed(Phase::Dispatch))?;
        let context = admit(state, &mut next, name)?;
        self.peer = next;
        self.context = context.clone();
        Ok(context)
    }
    async fn cleanup(&mut self) -> Result<(), FixtureError> {
        drop(self.pending.take());
        drop(self.state.take());
        // Closing the receiver releases bounded reply workers before joining
        // the peer; queued credentials are dropped, never serialized or logged.
        self.channels.events.close();
        while let Ok(_event) = self.channels.events.try_recv() {}
        self.peer.close().await?;
        self.home.remove()
    }
}

pub(super) fn admit(
    state: &mut State,
    peer: &mut peer::Peer,
    name: &str,
) -> Result<ConnectionContext, FixtureError> {
    let binding = vessel_client::book::BookEntry {
        address: "ws://127.0.0.1:0/rpc"
            .parse()
            .map_err(|_invalid| FixtureError::Protocol)?,
        token: None,
        credential: None,
        id: None,
    };
    let client = peer
        .client
        .take()
        .ok_or(FixtureError::Closed(Phase::Setup))?;
    let connected = state
        .admit_connection(name, &binding, client, Vec::new())
        .map_err(|_exhausted| FixtureError::Admission)?;
    let context = connected.context.clone();
    let _previous = state.clients.insert(name.to_owned(), connected);
    Ok(context)
}
fn answer(port: Port, result: Result<Response, FixtureError>) {
    let _closed = port.send(result);
}
fn prune(waiter: &mut Option<Port>) {
    if waiter.as_ref().is_some_and(oneshot::Sender::is_closed) {
        let _closed = waiter.take();
    }
}
