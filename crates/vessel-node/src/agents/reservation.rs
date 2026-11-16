//! Count admission belongs to Agents; a receipt attempt owns only its rollback.
use super::{Agents, Entry};
use std::sync::Arc;
use vessel_proto::{AgentSecret, SessionId};

#[cfg(test)]
mod launch;
#[cfg(test)]
pub(in crate::agents) use launch::CountLaunchState;
#[cfg(test)]
pub(crate) use launch::CreationCountCorrelation;

/// No credential, owner/grant or public session identity is replaced on `Drop`.
#[must_use = "retain the count reservation through commit or known refusal"]
pub(crate) struct AgentReservation<'a> {
    agents: &'a Agents,
    session: SessionId,
    marker: Arc<()>,
    secret: Option<AgentSecret>,
    rollback: bool,
    #[cfg(test)]
    launch_correlated: bool,
    #[cfg(test)]
    boundary: Option<(
        vessel_proto::WorkspaceOperationId,
        vessel_proto::creation::Revision,
    )>,
}

impl<'a> AgentReservation<'a> {
    pub(super) fn new(
        agents: &'a Agents,
        session: SessionId,
        marker: Arc<()>,
        secret: Option<AgentSecret>,
    ) -> Self {
        Self {
            agents,
            session,
            marker,
            secret,
            rollback: true,
            #[cfg(test)]
            launch_correlated: false,
            #[cfg(test)]
            boundary: None,
        }
    }

    /// Legacy launch already has explicit forget/retire paths; retain them.
    pub(super) fn retain_legacy(mut self) -> Option<AgentSecret> {
        self.rollback = false;
        self.secret.take()
    }

    /// Caller supplies the exact Store commit acknowledgement before spawn;
    /// this guard checks the resource marker and its own local entry identity.
    /// There is no generic automatic rollback after this boundary. A real
    /// driver must retain this guard in its tracked task, never its reply.
    #[cfg(test)]
    pub(crate) fn begin(
        &mut self,
        receipt: &vessel_proto::creation::Receipt,
    ) -> Result<(), vessel_proto::creation::CreationError> {
        use vessel_proto::creation::{CreationError, Resource, Stage};
        if self.boundary.is_some()
            || receipt.in_flight() != Some(Stage::Resource)
            || receipt.snapshot().reserved.resource
                != Some(Resource::Agent {
                    session: self.session,
                })
        {
            return Err(CreationError::InvalidTransition);
        }
        let mut entries = self.agents.lock();
        let entry = entries
            .get_mut(&self.session)
            .filter(|entry| self.owns_pending(entry))
            .ok_or(CreationError::InvalidTransition)?;
        if entry.creation_launch != CountLaunchState::Reserved {
            return Err(CreationError::InvalidTransition);
        }
        let begin = (
            receipt.snapshot().reserved.operation,
            receipt.snapshot().revision,
        );
        entry.creation_launch = CountLaunchState::Begun {
            operation: begin.0,
            revision: begin.1,
        };
        self.boundary = Some(begin);
        self.rollback = false;
        Ok(())
    }

    /// Caller must classify proven no-spawn refusal, then supply its exact
    /// committed failure acknowledgement. Generic process/task errors are not
    /// such proof. A registered/replaced/credential-used entry cannot roll back.
    #[cfg(test)]
    pub(crate) fn refused(
        &mut self,
        receipt: &vessel_proto::creation::Receipt,
    ) -> Result<(), vessel_proto::creation::CreationError> {
        use vessel_proto::creation::{Certainty, CreationError, Phase, Resource, Stage};
        let Some((operation, begin)) = self.boundary else {
            return Err(CreationError::InvalidTransition);
        };
        if operation != receipt.snapshot().reserved.operation
            || begin.next()? != receipt.snapshot().revision
            || receipt.in_flight().is_some()
            || receipt.snapshot().reserved.resource
                != Some(Resource::Agent {
                    session: self.session,
                })
            || !matches!(
                receipt.snapshot().phase,
                Phase::Failed {
                    stage: Stage::Resource,
                    certainty: Certainty::Known,
                    ..
                }
            )
        {
            return Err(CreationError::InvalidTransition);
        }
        let mut entries = self.agents.lock();
        let entry = entries
            .get_mut(&self.session)
            .filter(|entry| self.owns_pending(entry))
            .ok_or(CreationError::InvalidTransition)?;
        if entry.creation_launch
            != (CountLaunchState::Begun {
                operation,
                revision: begin,
            })
        {
            return Err(CreationError::InvalidTransition);
        }
        entry.creation_launch = CountLaunchState::Reserved;
        self.rollback = true;
        Ok(())
    }

    fn owns_pending(&self, entry: &Entry) -> bool {
        Arc::ptr_eq(&entry.reservation, &self.marker)
            && entry.session.is_none()
            && entry.connections.is_empty()
    }
}

impl Drop for AgentReservation<'_> {
    fn drop(&mut self) {
        if !self.rollback {
            return;
        }
        let mut entries = self.agents.lock();
        if entries
            .get(&self.session)
            .is_some_and(|entry| self.owns_pending(entry))
        {
            drop(entries.remove(&self.session));
        }
    }
}

#[cfg(test)]
mod tests;
