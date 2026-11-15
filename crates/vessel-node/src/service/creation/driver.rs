//! Test-only orchestration frame for the still-unavailable registry/runtime adapter.
//! This is not registered RPC behavior: real admission must first supply the
//! bounded principal/count/start-lease and generation fencing from ADR 0091.
use std::future::Future;
use vessel_proto::creation::{
    Advance, Certainty, CreationError, Fingerprint, FirstInput, Phase, Receipt, Reservation,
    Snapshot, Stage, WorkspaceClaim,
};
use vessel_proto::{ErrorCode, WorkspaceOperationId};
use vessel_store::{Store, StoreError};

const MAX_STEPS: usize = 4; // workspace, resource, first input, final settlement
#[derive(Clone, Copy)]
enum StepFailure {
    Refused(ErrorCode),
    Uncertain(ErrorCode),
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum StageReadiness {
    Ready,
    Deferred,
}
trait Runtime {
    fn fingerprint(&self) -> Fingerprint;
    /// Preparation/approval may be pending, without admitting an effect. The
    /// real first-input owner supplies this; receipt metadata is not acceptance.
    fn readiness(&self, stage: Stage, reserved: &Reservation) -> StageReadiness;
    /// A real adapter rechecks current authority immediately before effects.
    fn execute(
        &self,
        stage: Stage,
        reserved: &Reservation,
    ) -> impl Future<Output = Result<(), StepFailure>> + Send;
}
async fn drive(
    store: &Store,
    operation: WorkspaceOperationId,
    runtime: &impl Runtime,
) -> Result<Snapshot, StoreError> {
    let mut receipt = store
        .creation_operation(operation)
        .await?
        .ok_or(CreationError::NotFound)?;
    if receipt.fingerprint() != runtime.fingerprint() {
        return Err(CreationError::IntentConflict.into());
    }
    for _ in 0..MAX_STEPS {
        if matches!(
            receipt.snapshot().phase,
            Phase::Completed
                | Phase::Failed {
                    certainty: Certainty::Unknown,
                    ..
                }
        ) {
            return Ok(receipt.snapshot().clone());
        }
        // A caller cannot attach effects to another runner's committed marker.
        if receipt.in_flight().is_some() {
            return Err(CreationError::InvalidTransition.into());
        }
        let Some(stage) = next(&receipt) else {
            return store
                .advance_creation(operation, receipt.snapshot().revision, Advance::Complete)
                .await
                .map(|receipt| receipt.snapshot().clone());
        };
        if runtime.readiness(stage, &receipt.snapshot().reserved) == StageReadiness::Deferred {
            // A driver may settle/release its active permit while bounded input
            // preparation continues elsewhere. No Begin/effect is implied by
            // ResourceReady, and a later input uses the same child/message id.
            return Ok(receipt.snapshot().clone());
        }
        // Await the actual committed writer acknowledgement BEFORE dispatch.
        receipt = store
            .advance_creation(
                operation,
                receipt.snapshot().revision,
                Advance::Begin(stage),
            )
            .await?;
        let outcome = runtime.execute(stage, &receipt.snapshot().reserved).await;
        let update = match outcome {
            Ok(()) => Advance::Finished(stage),
            Err(StepFailure::Refused(code)) => Advance::Fail {
                code,
                certainty: Certainty::Known,
            },
            Err(StepFailure::Uncertain(code)) => Advance::Fail {
                code,
                certainty: Certainty::Unknown,
            },
        };
        receipt = store
            .advance_creation(operation, receipt.snapshot().revision, update)
            .await?;
        if outcome.is_err() {
            return Ok(receipt.snapshot().clone());
        }
    }
    Err(CreationError::InvalidTransition.into())
}
fn next(receipt: &Receipt) -> Option<Stage> {
    let snapshot = receipt.snapshot();
    match snapshot.phase {
        Phase::Accepted => Some(match snapshot.reserved.workspace_claim {
            WorkspaceClaim::Create => Stage::Workspace,
            WorkspaceClaim::Existing => Stage::Resource,
        }),
        Phase::WorkspaceReady => snapshot.reserved.resource.map(|_| Stage::Resource),
        Phase::ResourceReady => match &snapshot.reserved.first_input {
            FirstInput::Absent => None,
            FirstInput::Present | FirstInput::Identified { .. } => Some(Stage::Prompt),
        },
        Phase::Failed {
            stage,
            certainty: Certainty::Known,
            ..
        } => Some(stage),
        Phase::PromptStarted
        | Phase::Completed
        | Phase::Failed {
            certainty: Certainty::Unknown,
            ..
        } => None,
    }
}

#[cfg(test)]
mod tests;
