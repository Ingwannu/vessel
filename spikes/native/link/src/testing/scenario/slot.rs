//! One bounded physical peer, with its own pending frame and request waiter.
use super::{FixtureError, Port, Response, answer, peer};
use tokio::sync::oneshot;
use vessel_app_view::connection::ConnectionContext;
use vessel_proto::Outcome;

pub(in crate::testing::scenario) struct Slot {
    pub(super) peer: peer::Peer,
    context: ConnectionContext,
    request_wait: Option<Port>,
    pending: Option<oneshot::Sender<Outcome>>,
}
impl Slot {
    pub(super) fn new(peer: peer::Peer, context: ConnectionContext) -> Self {
        Self {
            peer,
            context,
            request_wait: None,
            pending: None,
        }
    }
    pub(in crate::testing::scenario) fn context(&self) -> &ConnectionContext {
        &self.context
    }
    pub(super) fn prune(&mut self) {
        super::prune(&mut self.request_wait);
    }
    pub(super) fn wants_call(&self) -> bool {
        self.request_wait.is_some() && self.pending.is_none()
    }
    pub(super) async fn next_request(&mut self) -> Option<peer::Request> {
        match self.peer.requests.as_mut() {
            Some(requests) => requests.recv().await,
            None => None,
        }
    }
    pub(super) fn request(&mut self, port: Port) {
        if self.request_wait.is_some() || self.pending.is_some() {
            answer(port, Err(FixtureError::Full));
        } else {
            self.request_wait = Some(port);
        }
    }
    pub(super) fn received_call(&mut self, call: Option<peer::Request>) {
        if let Some(port) = self.request_wait.take() {
            match call {
                Some(peer::Request {
                    call,
                    answer: reply,
                }) => {
                    self.pending = Some(reply);
                    answer(port, Ok(Response::Call(Box::new(call))));
                }
                None => answer(port, Err(FixtureError::Closed(super::Phase::Read))),
            }
        }
    }
    pub(super) fn reply(&mut self, outcome: Outcome) -> Result<Response, FixtureError> {
        self.pending
            .take()
            .ok_or(FixtureError::NoPendingCall)
            .and_then(|pending| {
                pending
                    .send(outcome)
                    .map_err(|_discarded| FixtureError::Closed(super::Phase::Read))
            })
            .map(|()| Response::Done)
    }
    pub(super) fn cancel_pending(&mut self) {
        drop(self.pending.take());
    }
    pub(super) fn transcript(&self) -> Result<Response, FixtureError> {
        self.peer
            .calls
            .lock()
            .map_err(|_poison| FixtureError::Worker)
            .map(|calls| Response::Transcript(calls.clone()))
    }
}
