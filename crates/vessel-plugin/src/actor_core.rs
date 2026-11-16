//! Pure actor transitions: publication, correlation, revocation and admission.

use std::{collections::BTreeMap, num::NonZeroU64, sync::Arc};

use tokio::sync::{OwnedSemaphorePermit, mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use vessel_proto::plugins::{
    ClientDocument, HelperCall, HelperPayload, HelperToNode, HookSubscriptions, InvocationId,
    NodeToHelper, PluginAddress, RuntimeGeneration,
};

use crate::runtime_error::{ProtocolViolation, RuntimeError};
use crate::runtime_types::{CallError, MAX_PENDING_CALLS, Request, RuntimeState, StateSender};

struct Pending {
    reply: Option<oneshot::Sender<Result<HelperPayload, CallError>>>,
    cancel: CancellationToken,
    deadline: tokio::time::Instant,
    state: PendingState,
    _bytes: OwnedSemaphorePermit,
    _count: OwnedSemaphorePermit,
}

#[derive(PartialEq, Eq)]
enum PendingState {
    Active,
    Cancelling,
}

pub(crate) struct Core {
    address: PluginAddress,
    client: Option<Arc<ClientDocument>>,
    hooks: HookSubscriptions,
    high: u64,
    pending: BTreeMap<InvocationId, Pending>,
    writer: mpsc::Sender<NodeToHelper>,
    published: StateSender,
}

impl Core {
    pub(crate) fn new(
        address: PluginAddress,
        writer: mpsc::Sender<NodeToHelper>,
        published: StateSender,
    ) -> Self {
        Self {
            address,
            client: None,
            hooks: HookSubscriptions::default(),
            high: 0,
            pending: BTreeMap::new(),
            writer,
            published,
        }
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.client.is_some()
    }

    pub(crate) fn admit(&mut self, request: Request) -> Result<(), RuntimeError> {
        if let Err(error) = self.validate_request(&request) {
            drop(request.reply.send(Err(error)));
            return Ok(());
        }
        let high = self
            .high
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .ok_or(ProtocolViolation::IdentityExhausted)?;
        let id = InvocationId::new(high);
        let message = NodeToHelper::Request {
            request: id,
            call: request.call,
        };
        match self.writer.try_send(message) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_message)) => {
                drop(request.reply.send(Err(CallError::Busy)));
                return Ok(());
            }
            Err(mpsc::error::TrySendError::Closed(_message)) => {
                return Err(RuntimeError::EndOfStream);
            }
        }
        self.high = high.get();
        let _previous = self.pending.insert(
            id,
            Pending {
                reply: Some(request.reply),
                cancel: request.cancel,
                deadline: request.deadline,
                state: PendingState::Active,
                _bytes: request.bytes,
                _count: request.count,
            },
        );
        Ok(())
    }

    fn validate_request(&self, request: &Request) -> Result<(), CallError> {
        if request.cancel.is_cancelled() || request.reply.is_closed() {
            return Err(CallError::Cancelled);
        }
        if request.deadline <= tokio::time::Instant::now() {
            return Err(CallError::Timeout);
        }
        let client = self.client.as_ref().ok_or(CallError::NotReady)?;
        if request.address != self.address {
            return Err(CallError::Stale);
        }
        if self.pending.len() >= MAX_PENDING_CALLS {
            return Err(CallError::Busy);
        }
        let registered = match &request.call {
            HelperCall::Invoke { method, .. } => client.methods().contains(method),
            HelperCall::Hook { name, .. } => self.hooks.contains(*name),
        };
        if !registered {
            return Err(CallError::Unregistered);
        }
        Ok(())
    }

    pub(crate) fn receive(&mut self, message: HelperToNode) -> Result<(), RuntimeError> {
        match message {
            HelperToNode::Ready {
                protocol: _,
                client,
                hooks,
            } => {
                if self.client.is_some() {
                    return Err(ProtocolViolation::Ordering.into());
                }
                self.client = Some(Arc::new(client));
                self.hooks = hooks;
                self.publish();
            }
            HelperToNode::Result { request, output } => self.settle(request, Ok(output))?,
            HelperToNode::Error {
                request,
                kind,
                diagnostic: _,
            } => self.settle(request, Err(CallError::Failed(kind)))?,
            HelperToNode::Fatal { diagnostic: _ } => return Err(RuntimeError::Fatal),
            HelperToNode::ClientChanged { client } => {
                let old = self.client.as_ref().ok_or(ProtocolViolation::Ordering)?;
                if old.as_ref() != &client {
                    self.revoke()?;
                    self.client = Some(Arc::new(client));
                    self.publish();
                }
            }
            HelperToNode::HooksChanged { hooks } => {
                if self.client.is_none() {
                    return Err(ProtocolViolation::Ordering.into());
                }
                if self.hooks != hooks {
                    self.revoke()?;
                    self.hooks = hooks;
                    self.publish();
                }
            }
        }
        Ok(())
    }

    fn publish(&self) {
        if let Some(client) = &self.client {
            drop(self.published.send_replace(RuntimeState::Running {
                address: self.address.clone(),
                client: Arc::clone(client),
                hooks: self.hooks.clone(),
            }));
        }
    }

    fn settle(
        &mut self,
        id: InvocationId,
        result: Result<HelperPayload, CallError>,
    ) -> Result<(), RuntimeError> {
        if self.client.is_none() || id.get() > self.high {
            return Err(ProtocolViolation::UnissuedRequest.into());
        }
        // A ready pipe and deadline may both be selected in the same turn.
        // Arbitration must not turn a late callback into successful work or
        // release its admission before the supervisor retires that helper.
        if self
            .pending
            .get(&id)
            .is_some_and(|pending| pending.deadline <= tokio::time::Instant::now())
        {
            return Err(RuntimeError::CallTimeout);
        }
        if let Some(mut pending) = self.pending.remove(&id)
            && pending.state == PendingState::Active
            && !pending.cancel.is_cancelled()
            && let Some(reply) = pending.reply.take()
        {
            drop(reply.send(result));
        }
        // Already retired ids are harmless late/duplicate replies. No unbounded
        // tombstone set is needed because only this actor allocates increasing ids.
        Ok(())
    }

    fn revoke(&mut self) -> Result<(), RuntimeError> {
        self.address.generation = RuntimeGeneration::random();
        for (id, pending) in &mut self.pending {
            if let Some(reply) = pending.reply.take() {
                drop(reply.send(Err(CallError::Stale)));
            }
            cancel_pending(&self.writer, *id, pending)?;
        }
        Ok(())
    }

    pub(crate) fn cancellations(&mut self) -> Result<(), RuntimeError> {
        for (id, pending) in &mut self.pending {
            if pending.cancel.is_cancelled()
                || pending
                    .reply
                    .as_ref()
                    .is_none_or(oneshot::Sender::is_closed)
            {
                if let Some(reply) = pending.reply.take() {
                    drop(reply.send(Err(CallError::Cancelled)));
                }
                cancel_pending(&self.writer, *id, pending)?;
            }
        }
        Ok(())
    }

    pub(crate) fn deadline(&self) -> Option<tokio::time::Instant> {
        self.pending.values().map(|pending| pending.deadline).min()
    }

    pub(crate) fn retire(&mut self, error: CallError) {
        drop(self.published.send_replace(RuntimeState::Stopped));
        for pending in self.pending.values_mut() {
            if let Some(reply) = pending.reply.take() {
                drop(reply.send(Err(error)));
            }
        }
        // Keep issued count/byte permits until the supervisor has reaped the
        // group; merely withdrawing its public capability does not stop code.
    }
}

fn cancel_pending(
    writer: &mpsc::Sender<NodeToHelper>,
    id: InvocationId,
    pending: &mut Pending,
) -> Result<(), RuntimeError> {
    if pending.state == PendingState::Active {
        writer
            .try_send(NodeToHelper::Cancel { request: id })
            .map_err(|_full_or_closed| ProtocolViolation::ControlQueueFull)?;
        pending.state = PendingState::Cancelling;
    }
    // Keep admission until an actual reply or permanent retirement of this
    // actor. A cleanup request receipt alone does not prove descendant death;
    // the node must refuse replacement while cleanup remains uncertain.
    Ok(())
}

#[cfg(test)]
#[path = "actor_core_tests.rs"]
mod tests;
