//! Staged fixed creation admission. A successful observation is never returned
//! as permission: the same guard performs count insertion and consumes the
//! concrete owner capsule. All preparation and eventual spawn stay outside it.
use super::{CapturedParentExecution, has_both};
use crate::agents::creation_launch::{Candidate, CandidateProblem, TrackedLaunch};
use crate::agents::{AGENT_SCOPES, AgentReservation, Agents, Entry};
use crate::error::ServiceError;
use crate::principal::Principal;
use std::collections::HashMap;
use std::sync::Arc;
use vessel_proto::SessionId;
use vessel_proto::timeline::FinishGrantCeiling;

#[derive(Debug, Clone, Copy)]
pub(crate) enum CreationCredentials {
    WithCredential,
    WithoutCredential,
}

#[derive(Debug)]
pub(crate) enum CreationFenceRefusal {
    Registry,
    ParentChanged,
    ParentUnavailable,
    Grant,
    CountChanged,
    Candidate(CandidateProblem),
    Runtime(vessel_session::SessionError),
    Reservation(ServiceError),
}

/// Failure retains the exact concrete input/start/metadata owner for settlement.
#[derive(Debug)]
pub(crate) struct RejectedCreation<'c> {
    pub(crate) reason: CreationFenceRefusal,
    pub(crate) candidate: Candidate<'c>,
}

impl Agents {
    /// First admission uses the exact current parent and the existing count owner.
    /// This is not the post-Begin launch permission; no provider effect occurs.
    pub(crate) fn reserve_captured_creation(
        &self,
        parent: &CapturedParentExecution,
        child: SessionId,
        credentials: CreationCredentials,
    ) -> Result<AgentReservation<'_>, CreationFenceRefusal> {
        let prepared = self
            .prepare_reservation(
                child,
                matches!(credentials, CreationCredentials::WithCredential),
            )
            .map_err(CreationFenceRefusal::Reservation)?;
        // Normalize the bound child's grant before locking. This value is not
        // authorization; the original/current capture is checked below.
        let principal = Principal::Agent {
            node: self.node,
            session: parent.stamp.session,
            scopes: AGENT_SCOPES.to_vec(),
        };
        let mut entries = self.lock();
        self.current_creation_parent(parent, &entries)?;
        self.reserve_prepared(&mut entries, &principal, prepared)
            .map_err(CreationFenceRefusal::Reservation)
    }

    /// After actual durable Begin ACK, validate and insert under ONE guard.
    /// No caller can turn an unlocked check into a later spawn permission.
    pub(crate) fn admit_captured_creation<'c>(
        &self,
        candidate: Candidate<'c>,
    ) -> Result<TrackedLaunch<'c>, RejectedCreation<'c>> {
        let mut entries = self.lock();
        let result = self.mark_creation_count(&candidate, &mut entries);
        if let Err(reason) = result {
            return Err(RejectedCreation { reason, candidate });
        }
        // The concrete Box moves before this guard drops. A transition cannot
        // interleave between current-parent validation and this insertion.
        Ok(candidate.insert())
    }

    fn current_creation_parent(
        &self,
        parent: &CapturedParentExecution,
        entries: &HashMap<SessionId, Entry>,
    ) -> Result<(), CreationFenceRefusal> {
        if !std::ptr::eq(self, Arc::as_ptr(&parent.agents)) || parent.stamp.node != self.node {
            return Err(CreationFenceRefusal::Registry);
        }
        let entry = entries
            .get(&parent.stamp.session)
            .ok_or(CreationFenceRefusal::ParentUnavailable)?;
        if entry.execution.changed.borrow().incarnation != parent.stamp.incarnation
            || parent.changed.borrow().incarnation != parent.stamp.incarnation
        {
            return Err(CreationFenceRefusal::ParentChanged);
        }
        if parent.changed.has_changed().is_err()
            || !entry
                .session
                .as_ref()
                .is_some_and(|handle| handle.id() == parent.stamp.session && !handle.has_ended())
        {
            return Err(CreationFenceRefusal::ParentUnavailable);
        }
        if parent.ceiling != FinishGrantCeiling::ReadOperate || !has_both(&entry.scopes) {
            return Err(CreationFenceRefusal::Grant);
        }
        Ok(())
    }

    fn mark_creation_count(
        &self,
        candidate: &Candidate<'_>,
        entries: &mut HashMap<SessionId, Entry>,
    ) -> Result<(), CreationFenceRefusal> {
        candidate
            .check_runtime()
            .map_err(CreationFenceRefusal::Runtime)?;
        self.current_creation_parent(candidate.capture(), entries)?;
        if !candidate.owner_matches(self) || candidate.count().child() != candidate.child() {
            return Err(CreationFenceRefusal::Registry);
        }
        candidate
            .check_deadline()
            .map_err(CreationFenceRefusal::Candidate)?;
        let child = entries
            .get_mut(&candidate.child())
            .ok_or(CreationFenceRefusal::CountChanged)?;
        if !candidate.count().admit_pending(self, child) {
            return Err(CreationFenceRefusal::CountChanged);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
