//! Correlation for one concrete launch candidate, never a parent grant.
//! Only the common Agents guard may compare it with the current pending entry.
use super::{AgentReservation, Agents, Entry};
use std::{fmt, sync::Arc};
use vessel_proto::creation::{CreationError, Revision};
use vessel_proto::{SessionId, WorkspaceOperationId};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::agents) enum CountLaunchState {
    #[default]
    Reserved,
    Begun {
        operation: WorkspaceOperationId,
        revision: Revision,
    },
    Admitted {
        operation: WorkspaceOperationId,
        revision: Revision,
    },
}

pub(crate) struct CreationCountCorrelation<'a> {
    registry: &'a Agents,
    child: SessionId,
    marker: Arc<()>,
    begin: (WorkspaceOperationId, Revision),
}

impl fmt::Debug for CreationCountCorrelation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreationCountCorrelation")
            .field("child", &self.child)
            .field("begin", &self.begin)
            .finish_non_exhaustive()
    }
}

impl<'a> AgentReservation<'a> {
    /// The tracked driver retains this guard. The capsule owns only correlation;
    /// dropping a reply or this value confers neither rollback nor launch rights.
    pub(crate) fn creation_correlation(
        &mut self,
    ) -> Result<CreationCountCorrelation<'a>, CreationError> {
        let begin = self.boundary.ok_or(CreationError::InvalidTransition)?;
        if self.rollback || self.launch_correlated {
            return Err(CreationError::InvalidTransition);
        }
        self.launch_correlated = true;
        Ok(CreationCountCorrelation {
            registry: self.agents,
            child: self.session,
            marker: Arc::clone(&self.marker),
            begin,
        })
    }
}

impl CreationCountCorrelation<'_> {
    pub(crate) const fn child(&self) -> SessionId {
        self.child
    }
    pub(crate) const fn begin(&self) -> (WorkspaceOperationId, Revision) {
        self.begin
    }
    /// Called ONLY while the existing Agents registry guard is held. No nested
    /// lock, I/O, Store read, callback or authority token is introduced here.
    pub(in crate::agents) fn matches_pending(&self, registry: &Agents, entry: &Entry) -> bool {
        std::ptr::eq(self.registry, registry)
            && Arc::ptr_eq(&self.marker, &entry.reservation)
            && entry.session.is_none()
            && entry.connections.is_empty()
            && entry.creation_launch
                == CountLaunchState::Begun {
                    operation: self.begin.0,
                    revision: self.begin.1,
                }
    }

    /// The common fence performs its current parent check and concrete capsule
    /// insertion under this SAME guard. This count transition alone grants no
    /// launch authority and must never be exposed as an unlocked permission.
    pub(in crate::agents) fn admit_pending(&self, registry: &Agents, entry: &mut Entry) -> bool {
        if !self.matches_pending(registry, entry) {
            return false;
        }
        entry.creation_launch = CountLaunchState::Admitted {
            operation: self.begin.0,
            revision: self.begin.1,
        };
        true
    }
}

#[cfg(test)]
mod tests;
