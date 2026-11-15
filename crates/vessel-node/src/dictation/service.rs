//! Product-facing dictation operations; registry and private causes stay internal.
use super::{
    prepare::Factory,
    quota::ConnectionBudget,
    registry::{Caller, Registry, Snapshot},
    secret_read::Reader,
    updates::{End, Event},
    wire,
};
use crate::{connection_life::ConnectionLife, settings::Settings};
use std::{fmt, sync::Arc};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::{
    Ack, WireError,
    dictation::{
        CancelDictation, ChunkDictation, DictationAck, DictationReason, DictationUpdate,
        FinishAccepted, FinishDictation, NextDictation, StartDictation,
    },
};
use vessel_rpc::Outbound;

/// A connection's aliases share only recording reservations, not a second identity.
#[derive(Clone)]
pub(crate) struct Resources(ConnectionBudget);
impl Default for Resources {
    fn default() -> Self {
        Self(ConnectionBudget::new())
    }
}
impl fmt::Debug for Resources {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DictationReservations")
    }
}

/// Calls retain only observers/reservations; they cannot keep a connection alive.
pub(crate) struct Context(Caller);
impl Context {
    /// Only the internal registry receives an already admitted observer bundle.
    pub(super) const fn caller(&self) -> &Caller {
        &self.0
    }

    /// Construct at the real connection boundary, never from request metadata.
    pub(crate) fn new(
        principal: &crate::principal::Principal,
        life: &ConnectionLife,
        resources: &Resources,
        outbound: Outbound,
    ) -> Result<Self, WireError> {
        if principal.agent().is_some() {
            return Err(WireError::new(
                vessel_proto::ErrorCode::Forbidden,
                "dictation is reserved for a person's client",
            ));
        }
        Ok(Self(Caller::new(life, resources.0.clone(), outbound)))
    }
}

/// One service owns one registry, TLS cache and shared secret-read gate.
#[derive(Clone)]
pub(crate) struct Service {
    registry: Registry,
    reader: Reader,
}
impl fmt::Debug for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DictationService")
    }
}
impl Service {
    /// Register actual teardown with the node's unbounded-by-grace work tracker.
    pub(crate) fn new(
        settings: Arc<Settings>,
        stop: &CancellationToken,
        tasks: &TaskTracker,
    ) -> Self {
        let reader = settings.dictation_reader();
        let registry = Registry::new(
            Factory::new(reader.clone()),
            Snapshot::Saved(settings),
            stop,
        );
        registry.track_shutdown(tasks);
        Self { registry, reader }
    }
    /// Settings and starts borrow the same bounded reader, never a new gate.
    pub(crate) const fn reader(&self) -> &Reader {
        &self.reader
    }

    /// Readiness is the start reply; no synthetic acknowledgement is emitted.
    pub(crate) async fn start(
        &self,
        context: &Context,
        request: StartDictation,
    ) -> Result<Ack, WireError> {
        self.registry
            .start(context.caller(), request.id, request.format)
            .await
            .map_err(|error| wire::refusal(&error))?;
        Ok(Ack)
    }
    /// Acknowledge only audio admitted by the exclusive stream actor.
    pub(crate) async fn chunk(
        &self,
        context: &Context,
        request: ChunkDictation,
    ) -> Result<DictationAck, WireError> {
        self.registry
            .chunk(context.caller(), &request.id, request.seq, request.pcm)
            .await
            .map(|through| DictationAck { through })
            .map_err(|error| wire::refusal(&error))
    }
    /// Return the first admitted finish's frozen budget, separately from text.
    pub(crate) async fn finish(
        &self,
        context: &Context,
        request: FinishDictation,
    ) -> Result<FinishAccepted, WireError> {
        self.registry
            .finish(context.caller(), &request.id, request.final_seq)
            .await
            .map(|within_ms| FinishAccepted { within_ms })
            .map_err(|error| wire::refusal(&error))
    }
    /// A canceled wait leaves both the stream and its unread update intact.
    pub(crate) async fn next(
        &self,
        context: &Context,
        request: NextDictation,
    ) -> Result<DictationUpdate, WireError> {
        let event = self
            .registry
            .next(context.caller(), &request.id)
            .await
            .map_err(|error| wire::refusal(&error))?;
        Ok(match event {
            Event::Partial(text) => DictationUpdate::Partial { text },
            Event::End(End::Final(text)) => DictationUpdate::Final { text },
            Event::End(End::Failed(error)) => DictationUpdate::Failed {
                reason: error.reason().unwrap_or(DictationReason::Upstream),
            },
        })
    }
    /// Wait for actual retirement, including nonabortable preparation.
    pub(crate) async fn cancel(
        &self,
        context: &Context,
        request: CancelDictation,
    ) -> Result<Ack, WireError> {
        self.registry
            .cancel(context.caller(), &request.id)
            .await
            .map_err(|error| wire::refusal(&error))?;
        Ok(Ack)
    }
}

#[cfg(test)]
mod tests;
