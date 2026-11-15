//! One real dispatch/event fixture. Tests script only framed calls and outcomes;
//! admission, leases, guards, clients and State remain private to this owner.
use super::{Home, peer, phase};
use crate::{Command, Event, State};
use tokio::sync::mpsc;
use vessel_app_view::connection::ConnectionContext;

pub(crate) struct Actor {
    state: State,
    peer: peer::Peer,
    events: mpsc::Receiver<Event>,
    internal: mpsc::Receiver<Command>,
    _commands: mpsc::Sender<Command>,
    context: ConnectionContext,
    _home: Home,
}
impl Actor {
    pub(crate) async fn new(name: &str) -> Self {
        let peer = peer::controlled(name).await;
        let home = Home::new();
        let (mut state, events) = super::state(&home);
        let (commands, internal) = mpsc::channel(crate::CHANNEL);
        state.commands = commands.downgrade();
        let binding = vessel_client::book::BookEntry {
            address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
            token: None,
            credential: None,
            id: None,
        };
        let connected = state
            .admit_connection(
                name,
                &binding,
                peer.client.as_ref().unwrap().clone(),
                Vec::new(),
            )
            .unwrap();
        let context = connected.context.clone();
        let _previous = state.clients.insert(name.to_owned(), connected);
        Self {
            state,
            peer,
            events,
            internal,
            _commands: commands,
            context,
            _home: home,
        }
    }
    pub(crate) fn context(&self) -> ConnectionContext {
        self.context.clone()
    }
    pub(crate) async fn dispatch(&mut self, command: Command) {
        phase("actual actor command dispatch", self.state.handle(command)).await;
    }
    pub(crate) async fn request(&mut self) -> peer::Request {
        phase(
            "actual framed call",
            self.peer.requests.as_mut().unwrap().recv(),
        )
        .await
        .unwrap()
    }
    pub(crate) async fn next_event(&mut self) -> Event {
        phase("actual actor event", async {
            loop {
                tokio::select! {
                    biased;
                    Some(command) = self.internal.recv() => self.state.handle(command).await,
                    Some(event) = self.events.recv() => return event,
                }
            }
        })
        .await
    }
    pub(crate) async fn retire(&mut self) {
        self.dispatch(Command::RetireConnection(self.context.tag()))
            .await;
    }
    pub(crate) async fn close_peer(&mut self) {
        if let Some(task) = self.peer.task.take() {
            task.abort();
            let _ended = phase("framed peer task cancellation", task).await;
        }
        phase(
            "actual transport close",
            self.peer.client.as_ref().unwrap().closed(),
        )
        .await;
    }
}
