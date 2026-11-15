//! Staged empty-workspace adapter joins the real receipt and backing owners.
//! Source/visibility must already be resolved by the server. No RPC is activated.
use super::driver::{self, Runtime, StageReadiness, StepFailure};
use crate::workspace::{Workspaces, provisioning::Provisioned};
use sha2::{Digest as _, Sha256};
use std::sync::Arc;
use vessel_proto::creation::{
    CreationError, Fingerprint, FirstInput, Kind, Reservation, Snapshot, Stage, WorkspaceClaim,
};
use vessel_proto::registry::WorkspaceIsolation;
use vessel_proto::workspace::SetupChoice;
use vessel_proto::{BlobHash, Creator, ErrorCode, WorkspaceOperationId};
use vessel_store::{Store, StoreError, WorkspaceProvision};

/// Only explicit backing-only work is complete without the separate setup owner.
pub(crate) struct PreparedEmpty {
    intent: WorkspaceProvision,
    fingerprint: Fingerprint,
}
impl PreparedEmpty {
    pub(crate) fn new(intent: WorkspaceProvision) -> Result<Self, StoreError> {
        if matches!(
            intent.isolation,
            WorkspaceIsolation::Worktree {
                setup: SetupChoice::Run,
                ..
            }
        ) {
            return Err(CreationError::InvalidTransition.into());
        }
        let encoded = serde_json::to_vec(&intent).map_err(StoreError::Encode)?;
        let mut hash = Sha256::new();
        // Distinguish this backing-only contract from future resource/setup intent.
        hash.update(b"vessel-empty-workspace-v1\0");
        hash.update(encoded);
        Ok(Self {
            intent,
            fingerprint: Fingerprint::new(BlobHash::from_bytes(hash.finalize().into())),
        })
    }
    pub(crate) const fn fingerprint(&self) -> Fingerprint {
        self.fingerprint
    }
}

struct Empty<'a, F> {
    workspaces: &'a Arc<Workspaces>,
    prepared: &'a PreparedEmpty,
    entry_check: F,
}
impl<F: Fn() -> Result<Creator, ErrorCode> + Sync> Runtime for Empty<'_, F> {
    fn fingerprint(&self) -> Fingerprint {
        self.prepared.fingerprint
    }
    fn readiness(&self, _stage: Stage, _reserved: &Reservation) -> StageReadiness {
        StageReadiness::Ready
    }
    async fn execute(&self, stage: Stage, reserved: &Reservation) -> Result<(), StepFailure> {
        if stage != Stage::Workspace {
            return Err(StepFailure::Refused(ErrorCode::InvalidRequest));
        }
        // Supplied by the future admitted connection/controller, never request
        // data. This checks entry AFTER Begin. It is NOT a retained authority
        // fence across the provisioning awaits; the live controller still owes it.
        let creator = (self.entry_check)().map_err(StepFailure::Refused)?;
        let result = self
            .workspaces
            .provision_workspace(creator, reserved.operation, self.prepared.intent.clone())
            .await
            .map_err(|_error| StepFailure::Uncertain(ErrorCode::Internal))?;
        match result {
            Provisioned::Ready(descriptor) if descriptor.id == reserved.workspace => Ok(()),
            Provisioned::Refused {
                certainty: vessel_proto::creation::Certainty::Known,
                ..
            }
            | Provisioned::NeedsApproval(_) => Err(StepFailure::Refused(ErrorCode::Conflict)),
            Provisioned::Ready(_)
            | Provisioned::Refused {
                certainty: vessel_proto::creation::Certainty::Unknown,
                ..
            }
            | Provisioned::InProgress(_)
            | Provisioned::Uncertain(_) => Err(StepFailure::Uncertain(ErrorCode::Conflict)),
        }
    }
}

pub(crate) async fn drive_empty(
    store: &Store,
    workspaces: &Arc<Workspaces>,
    operation: WorkspaceOperationId,
    prepared: &PreparedEmpty,
    entry_check: impl Fn() -> Result<Creator, ErrorCode> + Sync,
) -> Result<Snapshot, StoreError> {
    let receipt = store
        .creation_operation(operation)
        .await?
        .ok_or(CreationError::NotFound)?;
    let snapshot = receipt.snapshot();
    if snapshot.kind != Kind::Workspace
        || snapshot.reserved.workspace_claim != WorkspaceClaim::Create
        || snapshot.reserved.resource.is_some()
        || snapshot.reserved.first_input != FirstInput::Absent
    {
        return Err(CreationError::InvalidTransition.into());
    }
    driver::drive(
        store,
        operation,
        &Empty {
            workspaces,
            prepared,
            entry_check,
        },
    )
    .await
}
