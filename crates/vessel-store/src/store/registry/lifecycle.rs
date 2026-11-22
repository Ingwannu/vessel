//! Exact registry drain and retirement using the existing writer and watch.

use chrono::{DateTime, Utc};
use vessel_proto::registry::{RegistryRevision, WorkspaceMembership, WorkspaceResource};
use vessel_proto::{WorkspaceId, WorkspaceOperationId, WorktreeId};

use super::{RegistryWrite, Store};
use crate::{
    ArchiveSettlement, ProvisionRetirement, RegistryArchive, StoreError, WorkspaceArchiveClaim,
    WorkspaceArchivePurpose, registry,
};

impl Store {
    /// Refuse legacy removal only when the exact backing is retained.
    /// # Errors
    /// The backing reference could not be read.
    pub async fn registry_worktree_reference(
        &self,
        worktree: WorktreeId,
    ) -> Result<Option<WorkspaceId>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| registry::worktree_reference(connection, worktree, None))
        })
        .await
    }

    /// Confirm an archive claimant owns this exact, exclusively held backing.
    /// # Errors
    /// The token or backing reference could not be read.
    pub async fn registry_archive_owns_worktree(
        &self,
        claim: WorkspaceArchiveClaim,
        worktree: WorktreeId,
    ) -> Result<bool, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| registry::archive_owns_backing(connection, claim, worktree))
        })
        .await
    }

    /// Read only explicit registry members, never path-inferred legacy membership.
    /// # Errors
    /// A registry member could not be read or decoded.
    pub async fn registry_members(
        &self,
        workspace: WorkspaceId,
    ) -> Result<Vec<WorkspaceMembership>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| registry::workspace_memberships(connection, workspace))
        })
        .await
    }

    /// Close start admission and mint the only token allowed to finish this archive.
    /// # Errors
    /// A new lifecycle, revision or existing claim prevents admission.
    pub async fn claim_registry_archive(
        &self,
        workspace: WorkspaceId,
        expected: RegistryRevision,
        purpose: WorkspaceArchivePurpose,
        at: DateTime<Utc>,
    ) -> Result<WorkspaceArchiveClaim, StoreError> {
        let claim = WorkspaceArchiveClaim {
            workspace,
            token: WorkspaceOperationId::random(),
            purpose,
        };
        self.registry_write(RegistryWrite::ClaimArchive {
            claim,
            expected,
            at,
        })
        .await?;
        Ok(claim)
    }

    /// Publish the outcome of the exact admitted archive, preserving current metadata.
    /// # Errors
    /// Missing/stale token, unresolved starts or storage failure.
    pub async fn settle_registry_archive(
        &self,
        claim: WorkspaceArchiveClaim,
        settlement: ArchiveSettlement,
    ) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::SettleArchive { claim, settlement })
            .await
    }

    /// Recover the latest claim or its deferred reason without deriving actor ownership.
    /// # Errors
    /// The claim could not be read or decoded.
    pub async fn registry_archive(
        &self,
        workspace: WorkspaceId,
    ) -> Result<Option<RegistryArchive>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| registry::archive_claim(connection, workspace))
        })
        .await
    }

    /// Retain a supervisor's actual join receipt across node restarts.
    /// # Errors
    /// The writer could not record it.
    pub async fn registry_member_joined(
        &self,
        lease: crate::WorkspaceStartLease,
    ) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::MemberJoined(lease)).await
    }

    /// Notify the registry watcher after an exact actor's runtime state changed.
    /// # Errors
    /// The registry writer failed to publish the notification.
    pub async fn notify_registry_member(
        &self,
        resource: WorkspaceResource,
    ) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::MemberChanged(resource))
            .await
    }

    /// Release a provision slot only after all of its ownership effects settled.
    /// # Errors
    /// A retained backing, start, caller pin or missing receipt prevented release.
    pub async fn retire_registry_workspace(
        &self,
        workspace: WorkspaceId,
        evidence: ProvisionRetirement,
        at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::Retire {
            workspace,
            evidence,
            at,
        })
        .await
    }
}
