//! Single fixture owner: all controls and App commands use the existing State.
use super::{
    Control, FixtureError, Home, Lifetime, PeerNodes, PeerSlot, Phase, Port, Reconnect, Response,
    failure, peer,
};
use crate::{Command, Event, State};
use tokio::sync::{mpsc, oneshot, watch};
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::machine_book::Name;
use vessel_proto::NodeId;
mod slot;
pub(super) use slot::Slot;

pub(super) struct Seed {
    pub state: State,
    pub primary: Slot,
    pub secondary: Option<Slot>,
    pub home: Home,
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
    primary: Slot,
    secondary: Option<Slot>,
    home: Home,
    channels: Channels,
    event_wait: Option<Port>,
    stop_ack: Option<Port>,
}
impl Driver {
    pub(super) fn new(seed: Seed, channels: Channels) -> Self {
        Self {
            state: Some(seed.state),
            primary: seed.primary,
            secondary: seed.secondary,
            home: seed.home,
            channels,
            event_wait: None,
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
            self.primary.prune();
            if let Some(secondary) = &mut self.secondary {
                secondary.prune();
            }
            prune(&mut self.event_wait);
            let wants_primary = self.primary.wants_call();
            let wants_secondary = self.secondary.as_ref().is_some_and(Slot::wants_call);
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
                call = self.primary.next_request(), if wants_primary => self.primary.received_call(call),
                call = async {
                    match self.secondary.as_mut() {
                        Some(secondary) => secondary.next_request().await,
                        None => None,
                    }
                }, if wants_secondary => {
                    if let Some(secondary) = &mut self.secondary { secondary.received_call(call); }
                },
                event = self.channels.events.recv(), if wants_event => {
                    if let Some(port) = self.event_wait.take() {
                        answer(port, event.map(|event| Response::Event(Box::new(event)))
                            .ok_or(FixtureError::Closed(Phase::Control)));
                    }
                },
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
            Control::Request(peer, port) => match self.slot_mut(peer) {
                Ok(slot) => slot.request(port),
                Err(error) => answer(port, Err(error)),
            },
            Control::Event(port) => {
                if self.event_wait.is_some() {
                    answer(port, Err(FixtureError::Full));
                } else {
                    self.event_wait = Some(port);
                }
            }
            Control::Reply(peer, outcome, port) => {
                let result = self.slot_mut(peer).and_then(|slot| slot.reply(*outcome));
                answer(port, result);
            }
            Control::Retire(peer, port) => {
                let result = match self.slot_mut(peer) {
                    Ok(slot) => {
                        let tag = slot.context().tag();
                        Box::pin(self.dispatch(Command::RetireConnection(tag)))
                            .await
                            .map(|()| Response::Done)
                    }
                    Err(error) => Err(error),
                };
                answer(port, result);
            }
            Control::Reconnect(peer, change, port) => {
                let result = Box::pin(self.reconnect(peer, change))
                    .await
                    .map(Response::Context);
                answer(port, result);
            }
            Control::ClosePeer(peer, port) => {
                let result = match self.slot_mut(peer) {
                    Ok(slot) => slot.peer.close().await.map(|()| Response::Done),
                    Err(error) => Err(error),
                };
                answer(port, result);
            }
            Control::Transcript(peer, port) => {
                let result = self.slot_mut(peer).and_then(|slot| slot.transcript());
                answer(port, result);
            }
            Control::Stop(port) => {
                self.stop_ack = Some(port);
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn slot_mut(&mut self, peer: PeerSlot) -> Result<&mut Slot, FixtureError> {
        match peer {
            PeerSlot::Primary => Ok(&mut self.primary),
            PeerSlot::Secondary => self.secondary.as_mut().ok_or(FixtureError::Protocol),
        }
    }
    async fn reconnect(
        &mut self,
        peer: PeerSlot,
        change: Reconnect,
    ) -> Result<ConnectionContext, FixtureError> {
        let original = self.slot_mut(peer)?.context().clone();
        // Irreversible original retirement precedes every new handshake await.
        Box::pin(self.dispatch(Command::RetireConnection(original.tag()))).await?;
        let slot = self.slot_mut(peer)?;
        slot.cancel_pending();
        slot.peer.close().await?;
        let id = match change {
            Reconnect::SameNode => original.node(),
            Reconnect::DifferentNode => NodeId::random(),
        };
        let name = original.machine().as_str();
        let mut next = peer::try_controlled(name, id).await?;
        let state = self
            .state
            .as_mut()
            .ok_or(FixtureError::Closed(Phase::Dispatch))?;
        let context = admit(state, &mut next, name)?;
        *self.slot_mut(peer)? = Slot::new(next, context.clone());
        Ok(context)
    }
    async fn cleanup(&mut self) -> Result<(), FixtureError> {
        self.primary.cancel_pending();
        if let Some(secondary) = &mut self.secondary {
            secondary.cancel_pending();
        }
        let device_calls = self.state.as_ref().map(|state| state.device_calls.clone());
        if let Some(state) = &self.state {
            // Invalidate BOTH original admissions before either close can await.
            for connected in state.clients.values() {
                connected.retire();
            }
        }
        drop(self.state.take());
        // Closing the receiver releases bounded reply workers before joining
        // the peer; queued credentials are dropped, never serialized or logged.
        self.channels.events.close();
        while let Ok(_event) = self.channels.events.try_recv() {}
        let primary = self.primary.peer.close().await;
        let secondary = match &mut self.secondary {
            Some(secondary) => secondary.peer.close().await,
            None => Ok(()),
        };
        let workers = if let Some(hub) = device_calls {
            failure::phase(Phase::Workers, hub.drained())
                .await
                .and_then(|drained| drained.map_err(|_closed| FixtureError::Worker))
        } else {
            Ok(())
        };
        let home = self.home.remove();
        primary.and(secondary).and(workers).and(home)
    }
}

pub(super) async fn setup(
    state: &mut State,
    name: &Name,
    second: Option<(&Name, PeerNodes)>,
) -> Result<(Slot, Option<Slot>), FixtureError> {
    let id = NodeId::random();
    let mut primary = peer::try_controlled(name.as_str(), id).await?;
    let mut secondary = match second {
        Some((name, nodes)) => {
            let id = match nodes {
                PeerNodes::SameNode => id,
                PeerNodes::DifferentNodes => NodeId::random(),
            };
            match peer::try_controlled(name.as_str(), id).await {
                Ok(peer) => Some(peer),
                Err(error) => {
                    if let Err(cleanup) = primary.close().await {
                        tracing::debug!(%cleanup, "fixture setup cleanup failed");
                    }
                    return Err(error);
                }
            }
        }
        None => None,
    };
    let admitted = (|| {
        let primary = admit(state, &mut primary, name.as_str())?;
        let secondary = match (&mut secondary, second) {
            (Some(peer), Some((name, _))) => Some(admit(state, peer, name.as_str())?),
            (Some(_), None) | (None, Some(_)) => return Err(FixtureError::Protocol),
            (None, None) => None,
        };
        Ok((primary, secondary))
    })();
    match admitted {
        Ok((context, next)) => {
            let secondary = match (secondary, next) {
                (Some(peer), Some(context)) => Some(Slot::new(peer, context)),
                (None, None) => None,
                (Some(_), None) | (None, Some(_)) => return Err(FixtureError::Protocol),
            };
            Ok((Slot::new(primary, context), secondary))
        }
        Err(error) => {
            for connected in state.clients.values() {
                connected.retire();
            }
            if let Err(cleanup) = primary.close().await {
                tracing::debug!(%cleanup, "fixture setup cleanup failed");
            }
            if let Some(peer) = &mut secondary {
                if let Err(cleanup) = peer.close().await {
                    tracing::debug!(%cleanup, "fixture setup cleanup failed");
                }
            }
            Err(error)
        }
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
