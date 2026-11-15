//! Staged workspace lifecycle workflow. Remove cfg(test) only when the reviewed
//! registry adapter and pending RPC wiring use these ownership-fenced operations.

mod contract;
mod restore;
mod terminals;
#[cfg(test)]
mod tests;

pub(crate) use contract::{ArchiveBackend, ArchiveOutcome, Backing, Claim, Failure, Member, Stage};
pub(crate) use restore::{Recovery, RecoveryAction, RecoveryReport, RestoreBackend, restore};

use std::time::Duration;
use vessel_proto::WorkspaceId;

/// Safety floor for one workflow, including every lease wait and side effect.
/// Timeout retains the durable non-active claim; it never proves cancellation.
pub(crate) const WORKFLOW_DEADLINE: Duration = Duration::from_secs(180);

/// Every partial failure keeps its original typed cause for boundary mapping.
#[derive(Debug)]
pub(crate) struct Report<E> {
    pub(crate) workspace: WorkspaceId,
    pub(crate) outcome: ArchiveOutcome<E>,
}

/// Claim before drain, drain before backing cleanup, commit only after proof.
pub(crate) async fn archive<B: ArchiveBackend>(
    backend: &B,
    workspace: WorkspaceId,
) -> Report<B::Error> {
    let outcome = match tokio::time::timeout(WORKFLOW_DEADLINE, execute(backend, workspace)).await {
        Ok(outcome) => outcome,
        Err(_deadline) => ArchiveOutcome::Incomplete(vec![Failure::Deadline]),
    };
    Report { workspace, outcome }
}

async fn execute<B: ArchiveBackend>(
    backend: &B,
    workspace: WorkspaceId,
) -> ArchiveOutcome<B::Error> {
    let claim = match backend.claim(workspace).await {
        Ok(Claim::Acquired(claim)) => claim,
        Ok(Claim::AlreadyArchived) => return ArchiveOutcome::AlreadyArchived,
        Err(source) => return incomplete(Stage::Claim, source),
    };
    let members = match quiesce(backend, &claim).await {
        Ok(members) => members,
        Err(failure) => return ArchiveOutcome::Incomplete(vec![failure]),
    };
    let failures = drain(backend, &claim, members).await;
    if !failures.is_empty() {
        return ArchiveOutcome::Incomplete(failures);
    }
    let backing = match backend.cleanup_backing(&claim).await {
        Ok(backing) => backing,
        Err(source) => return incomplete(Stage::Backing, source),
    };
    match backend.finish(&claim).await {
        Ok(()) => ArchiveOutcome::Archived { backing },
        Err(source) => incomplete(Stage::Commit, source),
    }
}

async fn quiesce<B: ArchiveBackend>(
    backend: &B,
    claim: &B::Claim,
) -> Result<Vec<Member>, Failure<B::Error>> {
    backend
        .stop_setup(claim)
        .await
        .map_err(|source| Failure::At {
            stage: Stage::Setup,
            member: None,
            source,
        })?;
    backend
        .settle_starts(claim)
        .await
        .map_err(|source| Failure::At {
            stage: Stage::Starts,
            member: None,
            source,
        })?;
    backend.members(claim).await.map_err(|source| Failure::At {
        stage: Stage::Members,
        member: None,
        source,
    })
}

async fn drain<B: ArchiveBackend>(
    backend: &B,
    claim: &B::Claim,
    mut members: Vec<Member>,
) -> Vec<Failure<B::Error>> {
    // Shared terminal shutdown can already have ended a listed terminal; the
    // backend is idempotent. Duplicate rows must never issue a second mutation.
    members.sort_unstable();
    members.dedup();
    let mut failures = Vec::new();
    for member in members {
        if let Err(source) = backend.stop_member(claim, member).await {
            failures.push(Failure::At {
                stage: Stage::Drain,
                member: Some(member),
                source,
            });
        }
    }
    failures
}
fn incomplete<E>(stage: Stage, source: E) -> ArchiveOutcome<E> {
    ArchiveOutcome::Incomplete(vec![Failure::At {
        stage,
        member: None,
        source,
    }])
}
