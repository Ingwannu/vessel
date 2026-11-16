//! Staged capture only (A10). The finish-admission wiring removes cfg(test)
//! after the target-actor capsule/current-grant fence is implemented and checked.
//! Observation is never permission to enqueue after this function releases its lock.

use super::{Agents, ExecutionInvalidation};
use crate::principal::Principal;
use std::sync::Arc;
use tokio::sync::watch;
use vessel_proto::Scope;
use vessel_proto::timeline::{FinishGrantCeiling, ParentExecutionStamp};

mod creation;
pub(crate) use creation::{CreationCredentials, CreationFenceRefusal, RejectedCreation};

/// A runtime-only capture; no serde constructor can manufacture its owner/watch.
#[derive(Debug)]
pub(crate) struct CapturedParentExecution {
    agents: Arc<Agents>,
    stamp: ParentExecutionStamp,
    ceiling: FinishGrantCeiling,
    changed: watch::Receiver<super::Change>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptureRefusal {
    UnsupportedPrincipal,
    ForeignNode,
    Unavailable,
    Grant,
}

/// Read-only cancellation evidence. Current does not authorize a later effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionObservation {
    Current,
    Invalidated(Option<ExecutionInvalidation>),
    Unavailable,
}

impl CapturedParentExecution {
    pub(crate) const fn stamp(&self) -> ParentExecutionStamp {
        self.stamp
    }

    pub(crate) const fn ceiling(&self) -> FinishGrantCeiling {
        self.ceiling
    }

    /// Wait on this original capture, never a replacement incarnation. The
    /// tracked owner supplies its named supervision deadline and retains any
    /// in-flight spawn through settlement; this notification grants no effect.
    pub(crate) async fn invalidated(&mut self) -> ExecutionObservation {
        if self.changed.changed().await.is_err() {
            return ExecutionObservation::Unavailable;
        }
        self.observation()
    }

    pub(crate) fn observation(&self) -> ExecutionObservation {
        let entries = self.agents.lock();
        let Some(entry) = entries.get(&self.stamp.session) else {
            return ExecutionObservation::Unavailable;
        };
        let change = *self.changed.borrow();
        let current = *entry.execution.changed.borrow();
        if change.incarnation != self.stamp.incarnation {
            return ExecutionObservation::Invalidated(change.cause);
        }
        if current.incarnation != self.stamp.incarnation {
            return ExecutionObservation::Unavailable;
        }
        if self.changed.has_changed().is_err()
            || !entry
                .session
                .as_ref()
                .is_some_and(|handle| handle.id() == self.stamp.session && !handle.has_ended())
            || !has_both(&entry.scopes)
        {
            return ExecutionObservation::Unavailable;
        }
        ExecutionObservation::Current
    }
}

impl Agents {
    /// The direct authenticated Agent path alone captures a runtime execution.
    /// This observes no parentage and grants no queue/Store/provider effect.
    pub(crate) fn capture_execution(
        self: &Arc<Self>,
        principal: &Principal,
    ) -> Result<CapturedParentExecution, CaptureRefusal> {
        let Principal::Agent {
            node,
            session,
            scopes,
        } = principal
        else {
            return Err(CaptureRefusal::UnsupportedPrincipal);
        };
        if *node != self.node {
            return Err(CaptureRefusal::ForeignNode);
        }
        let entries = self.lock();
        let entry = entries.get(session).ok_or(CaptureRefusal::Unavailable)?;
        if !entry
            .session
            .as_ref()
            .is_some_and(|handle| handle.id() == *session && !handle.has_ended())
        {
            return Err(CaptureRefusal::Unavailable);
        }
        if !has_both(scopes) || !has_both(&entry.scopes) {
            return Err(CaptureRefusal::Grant);
        }
        // Subscribe before reading, while invalidate/retire/replacement are
        // excluded by this same guard. No subscribe-after-snapshot gap exists.
        let changed = entry.execution.changed.subscribe();
        let incarnation = changed.borrow().incarnation;
        Ok(CapturedParentExecution {
            agents: Arc::clone(self),
            stamp: ParentExecutionStamp {
                node: self.node,
                session: *session,
                incarnation,
            },
            ceiling: FinishGrantCeiling::ReadOperate,
            changed,
        })
    }
}

fn has_both(scopes: &[Scope]) -> bool {
    scopes.contains(&Scope::Read) && scopes.contains(&Scope::Operate)
}

// SessionHandle-positive cases run in sessions/tests/execution_capture.rs,
// reusing that module's private fake fleet and named setup/cleanup safety owner.
