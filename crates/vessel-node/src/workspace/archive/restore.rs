//! Recovery is inspection plus exclusive restore, with no provider resume or setup.
use super::WORKFLOW_DEADLINE;
use std::{error::Error, future::Future};
use vessel_proto::WorkspaceId;

/// Inspection chooses a known action; an unknown/unavailable plan is never executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecoveryAction {
    Unarchive,
    RecreateBacking,
}
/// Conditions for which retrying blindly cannot recreate an unavailable source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unavailable {
    WorkspaceMissing,
    ProjectMissing,
    DirectoryMissing,
    SourceMissing,
    UnsupportedBacking,
    UncertainStarts,
}
/// A plan carries the registry adapter's inspected revision and original placement.
pub(crate) enum Recovery<P> {
    Ready(P),
    Unavailable(Unavailable),
}
/// Failed recovery retains a typed cause and the non-active durable claim.
#[derive(Debug)]
pub(crate) enum RecoveryReport<E> {
    Restored(RecoveryAction),
    Unavailable(Unavailable),
    Failed { stage: RecoveryStage, source: E },
    Uncertain,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecoveryStage {
    Inspect,
    Claim,
    Backing,
    Activate,
}
/// Opaque plans/claims keep canonical registry identity in its owning adapter.
pub(crate) trait RestoreBackend: Sync {
    type Plan: Send + Sync;
    type Claim: Send + Sync;
    type Error: Error + Send + Sync + 'static;
    fn inspect(
        &self,
        workspace: WorkspaceId,
    ) -> impl Future<Output = Result<Recovery<Self::Plan>, Self::Error>> + Send;
    /// CAS Archived to Restoring; two calls cannot create the same backing twice.
    fn claim_restore(
        &self,
        workspace: WorkspaceId,
        plan: &Self::Plan,
    ) -> impl Future<Output = Result<Self::Claim, Self::Error>> + Send;
    /// Reinspect and validate the original subdirectory, rolling back only newly made backing on failure.
    fn restore_backing(
        &self,
        claim: &Self::Claim,
        plan: &Self::Plan,
    ) -> impl Future<Output = Result<RecoveryAction, Self::Error>> + Send;
    /// Fresh CAS after backing proof; never resume archived members or rerun setup.
    fn activate(&self, claim: &Self::Claim)
    -> impl Future<Output = Result<(), Self::Error>> + Send;
}
pub(crate) async fn restore<B: RestoreBackend>(
    backend: &B,
    workspace: WorkspaceId,
) -> RecoveryReport<B::Error> {
    match tokio::time::timeout(WORKFLOW_DEADLINE, execute(backend, workspace)).await {
        Ok(report) => report,
        Err(_deadline) => RecoveryReport::Uncertain,
    }
}
async fn execute<B: RestoreBackend>(
    backend: &B,
    workspace: WorkspaceId,
) -> RecoveryReport<B::Error> {
    let plan = match backend.inspect(workspace).await {
        Ok(Recovery::Ready(plan)) => plan,
        Ok(Recovery::Unavailable(reason)) => return RecoveryReport::Unavailable(reason),
        Err(source) => {
            return RecoveryReport::Failed {
                stage: RecoveryStage::Inspect,
                source,
            };
        }
    };
    let claim = match backend.claim_restore(workspace, &plan).await {
        Ok(claim) => claim,
        Err(source) => {
            return RecoveryReport::Failed {
                stage: RecoveryStage::Claim,
                source,
            };
        }
    };
    let action = match backend.restore_backing(&claim, &plan).await {
        Ok(action) => action,
        Err(source) => {
            return RecoveryReport::Failed {
                stage: RecoveryStage::Backing,
                source,
            };
        }
    };
    match backend.activate(&claim).await {
        Ok(()) => RecoveryReport::Restored(action),
        Err(source) => RecoveryReport::Failed {
            stage: RecoveryStage::Activate,
            source,
        },
    }
}
