//! Best-effort metadata accounting; a cancelled paid attempt must not masquerade as no call.

use super::super::{accounting, config, metadata::Selection};
use super::client::ClientError;
use std::time::Instant;
use vessel_llm::ir;
use vessel_proto::{DurationMs, ModelCall, ModelFailure, ModelStop};
use vessel_session::SessionHandle;

pub(super) struct Attempt<'a> {
    handle: &'a SessionHandle,
    pending: Option<ModelCall>,
    started: Instant,
}
impl<'a> Attempt<'a> {
    pub(super) fn new(handle: &'a SessionHandle, selected: &Selection<'_>) -> Self {
        Self {
            handle,
            pending: Some(ModelCall {
                route: selected.route().name.clone(),
                model: selected.model().clone(),
                api: config::model_api(selected.provider().flavor.api()),
                usage: vessel_proto::TokenUsage::default(),
                stop: ModelStop::Error,
                failure: Some(ModelFailure::Truncated),
                duration_ms: DurationMs::default(),
            }),
            started: Instant::now(),
        }
    }

    pub(super) fn answer(
        mut self,
        usage: &ir::Usage,
        stop: Option<ir::StopReason>,
        failure: Option<ModelFailure>,
    ) {
        if let Some(call) = self.pending.as_mut() {
            call.usage = accounting::usage(usage);
            call.stop = stop.map_or(ModelStop::Error, accounting::stop_of);
            call.failure = failure;
        }
        self.publish();
    }

    pub(super) fn failed(mut self, error: &ClientError) {
        let failure = match error {
            ClientError::Deadline(_) => ModelFailure::Timeout,
            ClientError::Provider(error) => accounting::failure_class(error.class()),
            // Both checks precede the transport. There was no attempt to bill.
            ClientError::Policy | ClientError::ApiMismatch => {
                self.pending = None;
                return;
            }
        };
        if let Some(call) = self.pending.as_mut() {
            call.failure = Some(failure);
        }
        self.publish();
    }

    fn publish(&mut self) {
        if let Some(mut call) = self.pending.take() {
            call.duration_ms = DurationMs::from_duration(self.started.elapsed());
            self.handle.model_call(call);
        }
    }
}
impl Drop for Attempt<'_> {
    fn drop(&mut self) {
        // Cancellation knows neither finality nor counts. Preserve that fact
        // while the original run is live, using the actor's existing queue.
        self.publish();
    }
}

/// Preserve provider counts and reported stops even when content cannot name a session.
pub(super) const fn unusable() -> ModelFailure {
    accounting::failure_class(ir::FailureClass::Truncated)
}
