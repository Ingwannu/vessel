//! Archive uses exact membership rather than any association by directory.
//! Path occupancy still protects physical backing independently of membership.

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use vessel_proto::registry::{RegistryRevision, WorkspaceBacking, WorkspaceDescriptor, WorkspaceResource};
use vessel_proto::workspace::{KeptReason, RemoveWorktreeOutcome};
use vessel_proto::{SessionId, WorkspaceId};
use vessel_session::IdleStop;
use vessel_store::{ArchiveDefer, ArchiveSettlement, StoreError, WorkspaceArchiveClaim, WorkspaceArchivePurpose};

use crate::workspace::{Occupant, WorkspaceError, Workspaces};

/// Adapters to the standard session and terminal cleanup paths. Their
/// effect scope is an exact registry member.
pub(crate) trait WorkspaceActors: Send + Sync {
    fn live(&self, resource: WorkspaceResource) -> bool;

    fn occupied_folders(&self) -> Vec<(Occupant, PathBuf)>;

    fn stop_idle(
        &self,
        session: SessionId,
    ) -> impl Future<Output = Result<IdleStop, WorkspaceError>> + Send;

    fn stop_member(
        &self,
        resource: WorkspaceResource,
    ) -> impl Future<Output = Result<(), WorkspaceError>> + Send;
}

/// Background cleanup can defer without changing the run's turn result.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ArchiveResult {
    /// Lifecycle is Archived. A kept backing still retains its reservation.
    Archived { kept: Vec<KeptReason> },
    Deferred(ArchiveDefer),
}

impl Workspaces {
    /// Mint an exclusive archive claim at the admitted lifecycle revision
    /// before observing starts or running any process or backing effect.
    pub(crate) async fn archive_workspace(
        self: &Arc<Self>,
        id: WorkspaceId,
        expected: RegistryRevision,
        purpose: WorkspaceArchivePurpose,
        session: Option<SessionId>,
        actors: &impl WorkspaceActors,
    ) -> Result<ArchiveResult, WorkspaceError> {
        let claim = self.store.claim_registry_archive(id, expected, purpose, Utc::now()).await?;
        self.resume_workspace_archive(claim, session, actors).await
    }

    /// The caller owns the live orchestration or the single startup recovery.
    /// A recovery token must be confirmed before touching any resource.
    pub(crate) async fn resume_workspace_archive(
        self: &Arc<Self>,
        claim: WorkspaceArchiveClaim,
        session: Option<SessionId>,
        actors: &impl WorkspaceActors,
    ) -> Result<ArchiveResult, WorkspaceError> {
        if !self.store.registry_archive(claim.workspace).await?.is_some_and(|current| {
            current.deferred.is_none() && current.claim == claim
        }) {
            return Err(StoreError::RegistryConflict.into());
        }
        self.drain_workspace_starts(claim.workspace).await?;
        let record = self.store.registry_workspace(claim.workspace).await?
            .ok_or(StoreError::RegistryMissing(claim.workspace))?;
        let members = self.store.registry_members(claim.workspace).await?;
        match claim.purpose {
            WorkspaceArchivePurpose::Explicit => {
                for member in members {
                    actors.stop_member(member.resource).await?;
                }
            }
            WorkspaceArchivePurpose::Automatic => {
                let target = session.map(|session| WorkspaceResource::Session { session });
                if let Some(session) = session
                    && members.iter().any(|member| Some(member.resource) == target)
                    && actors.stop_idle(session).await? == IdleStop::Busy
                {
                    return self.defer_archive(claim, ArchiveDefer::BusySession).await;
                }
                if members.iter().any(|member| Some(member.resource) != target && actors.live(member.resource)) {
                    return self.defer_archive(claim, ArchiveDefer::OtherMembers).await;
                }
            }
        }
        let kept = self.archive_backing(claim, &record.descriptor, actors).await?;
        self.store
            .settle_registry_archive(claim, ArchiveSettlement::Archived { at: Utc::now() })
            .await?;
        Ok(ArchiveResult::Archived { kept })
    }

    async fn defer_archive(
        &self,
        claim: WorkspaceArchiveClaim,
        reason: ArchiveDefer,
    ) -> Result<ArchiveResult, WorkspaceError> {
        self.store
            .settle_registry_archive(claim, ArchiveSettlement::Deferred(reason))
            .await?;
        Ok(ArchiveResult::Deferred(reason))
    }

    async fn drain_workspace_starts(&self, id: WorkspaceId) -> Result<(), WorkspaceError> {
        let mut wake = self.store.registry_changes();
        loop {
            let pending = self.store.pending_registry_starts(id).await?;
            if pending.is_empty() {
                return Ok(());
            }
            if pending.iter().any(|start| start.state == vessel_store::RegistryStartState::Uncertain) {
                return Err(WorkspaceError::Conflict(
                    "an unconfirmed workspace start must be reconciled".to_owned(),
                ));
            }
            tokio::select! {
                result = wake.changed() => result.map_err(|_| StoreError::RegistryConflict)?,
                () = self.stop.cancelled() => return Err(WorkspaceError::Conflict(
                    "the node stopped while waiting for workspace starts".to_owned(),
                )),
            }
        }
    }

    async fn archive_backing(
        self: &Arc<Self>,
        claim: WorkspaceArchiveClaim,
        workspace: &WorkspaceDescriptor,
        actors: &impl WorkspaceActors,
    ) -> Result<Vec<KeptReason>, WorkspaceError> {
        let WorkspaceBacking::Managed { worktree } = workspace.backing else {
            return Ok(Vec::new());
        };
        match self
            .remove_registry_backing(claim, worktree, || actors.occupied_folders())
            .await
        {
            Ok(RemoveWorktreeOutcome::Removed { .. }) => Ok(Vec::new()),
            Ok(RemoveWorktreeOutcome::Kept { reasons, .. }) => Ok(reasons),
            Err(WorkspaceError::RegistryWorktreeOwned(_)) => Ok(vec![KeptReason::InUse]),
            Err(WorkspaceError::UnknownWorktree(_))
                if tokio::fs::try_exists(&workspace.path).await.is_ok_and(|exists| !exists) =>
            {
                // The worktree row is removed only after Git confirms removal.
                Ok(Vec::new())
            }
            Err(error) => Err(error),
        }
    }
}
