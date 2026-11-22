//! A session that starts again in an independent workspace holds the same
//! exact lifecycle lease as a first start. Older unowned sessions use the
//! ordinary reopen path.

use vessel_proto::registry::WorkspaceResource;
use vessel_store::{RegistryResourceReadiness, StoreError, WorkspaceStartLease};

use super::Sessions;
use crate::error::ServiceError;

impl Sessions {
    pub(super) async fn record_workspace_join(&self, session: vessel_proto::SessionId) {
        let resource = WorkspaceResource::Session { session };
        let result = match self.store.registry_resource_run(resource).await {
            Ok(Some(run)) => self.store.registry_member_joined(run.lease).await,
            Ok(None) => Ok(()),
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            tracing::warn!(%session, %error, "cannot retain a workspace actor's join proof");
        }
    }

    pub(super) async fn admit_workspace_reopen(
        &self,
        resource: WorkspaceResource,
    ) -> Result<Option<WorkspaceStartLease>, ServiceError> {
        let Some(previous) = self.store.registry_resource_run(resource).await? else {
            return Ok(None);
        };
        let workspace = previous.lease.workspace();
        let current = self
            .store
            .registry_workspace(workspace)
            .await?
            .ok_or(StoreError::RegistryMissing(workspace))?;
        let expected = current.descriptor.revision;
        let lease = match previous.readiness {
            RegistryResourceReadiness::Ready => {
                self.store
                    .claim_registry_restart(previous.lease, expected)
                    .await?
            }
            RegistryResourceReadiness::Failed => {
                self.store
                    .claim_registry_start(workspace, resource, expected)
                    .await?
            }
            RegistryResourceReadiness::Pending | RegistryResourceReadiness::Uncertain => {
                return Err(StoreError::RegistryConflict.into());
            }
        };
        Ok(Some(lease))
    }
}
