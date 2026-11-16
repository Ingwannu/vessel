//! Typed staged service calls; the shared preflight refuses before sending.
//! No URL opener, environment injection or private settings writer lives here.

use vessel_proto::workspace_services::{
    GetServiceSettings, GetWorkspaceServiceEnv, RestartWorkspaceScript, ServiceEnvPlan,
    ServiceSettingsOutcome, ServicesAdminSettings, SetServiceExposure, SetServiceGateway,
    StartWorkspaceScript, StopWorkspaceScript, WorkspaceScriptOutcome, WorkspaceScriptQuery,
    WorkspaceScriptQueryResult,
};
use vessel_rpc::CallError;

use crate::NodeClient;

impl NodeClient {
    /// Bounded workspace observations; no cached source means typed unavailable.
    ///
    /// # Errors
    /// Unassigned/staged calls refuse before sending; otherwise see [`CallError`].
    pub async fn workspace_scripts(
        &self,
        request: WorkspaceScriptQuery,
    ) -> Result<WorkspaceScriptQueryResult, CallError> {
        self.call(request).await
    }

    /// Starting still requires the node's current canonical approval/start lease.
    ///
    /// # Errors
    /// Unassigned/staged calls refuse before effects; otherwise see [`CallError`].
    pub async fn start_workspace_script(
        &self,
        request: StartWorkspaceScript,
    ) -> Result<WorkspaceScriptOutcome, CallError> {
        self.call(request).await
    }

    /// An admission reply does not claim actual owned cleanup has completed.
    ///
    /// # Errors
    /// Unassigned/staged calls refuse before sending; otherwise see [`CallError`].
    pub async fn stop_workspace_script(
        &self,
        request: StopWorkspaceScript,
    ) -> Result<WorkspaceScriptOutcome, CallError> {
        self.call(request).await
    }

    /// The node must join the predecessor and acquire its exact restart lease.
    ///
    /// # Errors
    /// Unassigned/staged calls refuse before sending; otherwise see [`CallError`].
    pub async fn restart_workspace_script(
        &self,
        request: RestartWorkspaceScript,
    ) -> Result<WorkspaceScriptOutcome, CallError> {
        self.call(request).await
    }

    /// Read only already committed generated fields; never prepare a helper plan.
    ///
    /// # Errors
    /// Unassigned/staged calls refuse before sending; otherwise see [`CallError`].
    pub async fn workspace_service_env(
        &self,
        request: GetWorkspaceServiceEnv,
    ) -> Result<ServiceEnvPlan, CallError> {
        self.call(request).await
    }

    /// Dedicated direct Admin projection, distinct from ordinary node Read.
    ///
    /// # Errors
    /// Unassigned/staged calls refuse before sending; otherwise see [`CallError`].
    pub async fn service_settings(&self) -> Result<ServicesAdminSettings, CallError> {
        self.call(GetServiceSettings {}).await
    }

    /// A gateway choice alone cannot grant any service anonymous publication.
    ///
    /// # Errors
    /// Unassigned/staged calls refuse before sending; otherwise see [`CallError`].
    pub async fn set_service_gateway(
        &self,
        request: SetServiceGateway,
    ) -> Result<ServiceSettingsOutcome, CallError> {
        self.call(request).await
    }

    /// Explicit consent cannot replace missing canonical incarnation or run proof.
    ///
    /// # Errors
    /// Unassigned/staged calls refuse before sending; otherwise see [`CallError`].
    pub async fn set_service_exposure(
        &self,
        request: SetServiceExposure,
    ) -> Result<ServiceSettingsOutcome, CallError> {
        self.call(request).await
    }
}
