//! One retained destination and finite RPC waits; cancellation is never a retry.
use super::{Failure, FailureKind, Phase};
use crate::{NodeClient, Unary};
use std::{sync::Arc, time::Duration};
use tokio::{sync::watch, time::Instant};
use vessel_proto::NodeId;

pub(super) const READ: Duration = Duration::from_secs(45);
pub(super) const MUTATE: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Lifetime {
    Running,
    Cancelled,
}

pub(super) struct Endpoint {
    pub client: NodeClient,
    pub destination: NodeId,
    pub cancel: watch::Receiver<Lifetime>,
}
impl Endpoint {
    pub(super) fn cancelled(&self) -> bool {
        *self.cancel.borrow() == Lifetime::Cancelled
    }

    pub(super) async fn call<R: Unary>(
        &mut self,
        phase: Phase,
        request: R,
        deadline: Instant,
    ) -> Result<R::Response, Failure> {
        if self.cancelled() {
            return Err(Failure::new(phase, FailureKind::Cancelled));
        }
        // Biased cancellation prevents a ready reply from advancing another phase
        // after the sole owner's explicit stop. An admitted mutation is uncertain.
        tokio::select! {
            biased;
            _ = self.cancel.changed() => Err(Failure::new(phase, FailureKind::Cancelled)),
            result = tokio::time::timeout_at(deadline,
                self.client.call_on(self.destination, request)) => match result {
                Err(_) => Err(Failure::new(phase, FailureKind::Deadline)),
                Ok(Err(source)) => Err(Failure { phase, kind: FailureKind::Call,
                    code: source.code(), refusal: None, source: Some(Arc::new(super::state::Cause::Rpc(source))) }),
                Ok(Ok(reply)) => Ok(reply),
            }
        }
    }
}
impl Failure {
    pub(super) fn uncertain(&self) -> bool {
        match self.kind {
            FailureKind::Deadline | FailureKind::Cancelled => true,
            FailureKind::Call => self.source.as_deref().is_some_and(|source| match source {
                super::state::Cause::Rpc(error) => {
                    super::MutationError::of(error) == super::MutationError::Uncertain
                }
                super::state::Cause::Count(_) => false,
            }),
            FailureKind::Protocol
            | FailureKind::Worktree
            | FailureKind::PreflightStopped
            | FailureKind::Project
            | FailureKind::Provider
            | FailureKind::AgentEnded
            | FailureKind::Setup
            | FailureKind::Input
            | FailureKind::Source
            | FailureKind::Copy
            | FailureKind::CopyBudget => false,
        }
    }
}

// Finite built-in durations cannot overflow in practice. Fail closed (an
// immediate deadline) if the platform's monotonic clock cannot represent one.
pub(super) fn after(duration: Duration) -> Instant {
    Instant::now()
        .checked_add(duration)
        .unwrap_or_else(Instant::now)
}
