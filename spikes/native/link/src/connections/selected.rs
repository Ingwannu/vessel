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
    Call(#[source] CallError),
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
    client: vessel_client::NodeClient,
    context: ConnectionContext,
    validity: tokio::sync::watch::Receiver<Validity>,
    commands: tokio::sync::mpsc::WeakSender<Command>,
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
            client: connected.client.clone(),
            context: connected.context.clone(),
            validity: connected.validity(),
            commands: self.commands.clone(),
        })
    }
}
impl SelectedConnection {
    fn active(&self) -> Result<(), Refusal> {
        if self.client.node().id == self.context.node()
            && *self.validity.borrow() == Validity::Active
        {
            Ok(())
        } else {
            Err(Refusal::Changed)
        }
    }
    async fn fence<T>(&self, value: T) -> Result<T, Refusal> {
        self.active()?;
        let mut validity = self.validity.clone();
        tokio::select! {
            biased;
            _retired = validity.wait_for(|state| *state == Validity::Retired) => Err(Refusal::Changed),
            () = self.client.closed() => Err(Refusal::Closed),
            value = std::future::ready(value) => Ok(value),
        }
    }
    async fn guard<T>(
        &self,
        operation: impl Future<Output = Result<T, CallError>>,
    ) -> Result<T, RequestFailure> {
        self.active()?;
        let mut validity = self.validity.clone();
        let answer = tokio::select! {
            biased;
            _retired = validity.wait_for(|state| *state == Validity::Retired) => Err(Refusal::Changed.into()),
            () = self.client.closed() => Err(Refusal::Closed.into()),
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
                .send(Command::RetireConnection(self.context.tag()))
                .await;
        }
    }
    pub(crate) async fn call<R: Unary>(&self, request: R) -> Result<R::Response, RequestFailure> {
        self.guard(self.client.call(request)).await
    }
}

// Staged W12 attach/next boundary. The real resident-follow wiring removes
// this exact attribute; Unary remains a real Devices product path above.
#[cfg(test)]
mod follow;
