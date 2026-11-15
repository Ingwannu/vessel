//! One exact-alias preflight and before-poll guard for all feature RPCs.
//!
//! [Decision Log]
//! - Purpose: another alias, credential or connection must never receive intent.
//! - Constraint: actor admission and asynchronous first polling are different
//!   boundaries; a selected original client can retire between the two.
//! - Alternatives: per-area resolvers, first alias by `NodeId`, repeated lookups.
//! - Choice: one immutable selected client plus its irreversible validity;
//!   every typed call checks actual welcome identity before it can be polled.
//! - Why: callers retain tickets/workspace roots; this module never retargets.
//! - Cost: an already sent write may finish remotely; no replay is attempted.
use super::Validity;
use crate::{Command, State};
use std::future::Future;
use vessel_app_view::connection::{ConnectionContext, Refusal};
use vessel_client::{CallError, Unary};

/// Safe local refusal or the original typed client failure for edge projection.
#[derive(thiserror::Error)]
pub(crate) enum RequestFailure {
    #[error(transparent)]
    Refused(#[from] Refusal),
    // Preserve structured peer detail for each feature, never raw Display/Debug.
    #[error("the selected node refused the request")]
    // Do not expose a raw peer message through an automatic error source chain.
    Call(CallError),
}
impl std::fmt::Debug for RequestFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(why) => f.debug_tuple("Refused").field(why).finish(),
            Self::Call(_error) => f.write_str("Call(<redacted>)"),
        }
    }
}

/// An immutable original client; never resolved again through another alias.
#[derive(Clone)]
pub(crate) struct SelectedConnection {
    lease: std::sync::Arc<SelectedLease>,
    commands: tokio::sync::mpsc::WeakSender<Command>,
}
/// One canonical immutable lease anchored by Connected, not by queued events.
pub(crate) struct SelectedLease {
    client: vessel_client::NodeClient,
    context: ConnectionContext,
    validity: tokio::sync::watch::Receiver<Validity>,
}
impl SelectedLease {
    pub(super) fn new(
        client: vessel_client::NodeClient,
        context: ConnectionContext,
        validity: tokio::sync::watch::Receiver<Validity>,
    ) -> Self {
        Self {
            client,
            context,
            validity,
        }
    }
    pub(crate) fn validate(&self, expected: &ConnectionContext) -> Result<(), Refusal> {
        if !self.matches(expected) {
            return Err(Refusal::Changed);
        }
        // NodeClient::closed -> rpc::Connection::closed -> CancellationToken.
        // A single stack poll performs no I/O and drops its waiter on return.
        let mut closed = std::pin::pin!(self.client.closed());
        let mut task = std::task::Context::from_waker(std::task::Waker::noop());
        if closed.as_mut().poll(&mut task).is_ready() {
            Err(Refusal::Closed)
        } else {
            Ok(())
        }
    }
    pub(super) fn matches(&self, expected: &ConnectionContext) -> bool {
        self.context == *expected
            && self.client.node().id == expected.node()
            && self.validity.has_changed().is_ok()
            && *self.validity.borrow() == Validity::Active
    }
}
impl State {
    pub(crate) fn selected_connection(
        &self,
        expected: &ConnectionContext,
    ) -> Result<SelectedConnection, Refusal> {
        let connected = self
            .clients
            .get(expected.machine().as_str())
            .filter(|connected| connected.matches(expected))
            .ok_or(Refusal::Changed)?;
        Ok(SelectedConnection {
            lease: std::sync::Arc::clone(&connected.lease),
            commands: self.commands.clone(),
        })
    }
}
impl SelectedConnection {
    fn active(&self) -> Result<(), Refusal> {
        self.lease.validate(&self.lease.context)
    }
    async fn fence<T>(&self, value: T) -> Result<T, Refusal> {
        self.active()?;
        let mut validity = self.lease.validity.clone();
        tokio::select! {
            biased;
            _retired = validity.wait_for(|state| *state == Validity::Retired) => Err(Refusal::Changed),
            () = self.lease.client.closed() => Err(Refusal::Closed),
            value = std::future::ready(value) => Ok(value),
        }
    }
    async fn guard<T>(
        &self,
        operation: impl Future<Output = Result<T, CallError>>,
    ) -> Result<T, RequestFailure> {
        if let Err(why) = self.active() {
            if why == Refusal::Closed {
                self.report_closed().await;
            }
            return Err(why.into());
        }
        let mut validity = self.lease.validity.clone();
        let answer = tokio::select! {
            biased;
            _retired = validity.wait_for(|state| *state == Validity::Retired) => Err(Refusal::Changed.into()),
            () = self.lease.client.closed() => Err(Refusal::Closed.into()),
            answer = operation => answer.map_err(|error| match error {
                CallError::Disconnected => RequestFailure::Refused(Refusal::Closed),
                error @ (CallError::Remote(_) | CallError::NotInProtocol { .. }
                    | CallError::UnassignedProtocol { .. } | CallError::UnexpectedReply(_)
                    | CallError::RequestIdsExhausted) => RequestFailure::Call(error),
            }),
        };
        // Retirement that won while the awaited result completed must also
        // fence success, error and EOF before any caller can expose them.
        let answer = self
            .fence(answer)
            .await
            .map_err(RequestFailure::from)
            .and_then(|answer| answer);
        if matches!(answer, Err(RequestFailure::Refused(Refusal::Closed))) {
            self.report_closed().await;
        }
        answer
    }
    async fn report_closed(&self) {
        if let Some(commands) = self.commands.upgrade() {
            // The actor orders this retirement with its Machine snapshots.
            // Weak ownership lets dropping the UI close the command inbox.
            let _closed = commands
                .send(Command::RetireConnection(self.lease.context.tag()))
                .await;
        }
    }
    pub(crate) async fn call<R: Unary>(&self, request: R) -> Result<R::Response, RequestFailure> {
        self.guard(self.lease.client.call(request)).await
    }
    // Minted only by the actual result-envelope producer, from this SAME lease.
    pub(crate) fn delivery_guard(&self) -> crate::DeliveryGuard {
        crate::DeliveryGuard::from_lease(&self.lease)
    }
}

// Staged W12 attach/next boundary. The real resident-follow wiring removes
// this exact attribute; Unary remains a real Devices product path above.
#[cfg(test)]
mod follow;
// Real bounded result-envelope callers remove this literal staging attribute.
#[cfg(test)]
mod delivery_tests;
mod queue;
pub(crate) use queue::SendFailure;

#[cfg(test)]
mod privacy_tests {
    use super::*;
    #[test]
    fn peer_messages_cannot_escape_redaction_through_an_error_source() {
        let canary = "synthetic-invitation-credential";
        let failure = RequestFailure::Call(CallError::Remote(vessel_proto::WireError::new(
            vessel_proto::ErrorCode::Conflict,
            canary,
        )));
        assert!(!format!("{failure:?}").contains(canary));
        assert!(!failure.to_string().contains(canary));
        assert!(std::error::Error::source(&failure).is_none());
    }
}
