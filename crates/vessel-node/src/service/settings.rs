//! Protocol 12's `node.settings` and `node.set_settings` (ADR 0038): the
//! node's settings, kept and applied by `crate::settings`.

use vessel_proto::settings::{
    GetNodeSettings, NodeSettings, ReloadNodeSettings, ReloadedNodeSettings, SetNodeSettings,
    UnsetNodeSettings,
};
use vessel_proto::{ErrorCode, WireError};

use super::{Connection, NodeService};
use crate::settings::SettingsError;

impl NodeService {
    /// `node.settings`: the settings in force.
    pub(crate) async fn node_settings(
        &self,
        _: &Connection,
        _: GetNodeSettings,
    ) -> Result<NodeSettings, WireError> {
        Ok(self.settings.current().await)
    }

    /// `node.set_settings`: change, keep and apply what the request names.
    pub(crate) async fn set_node_settings(
        &self,
        _: &Connection,
        request: SetNodeSettings,
    ) -> Result<NodeSettings, WireError> {
        self.settings
            .change(request)
            .await
            .map_err(|error| failure(&error))
    }

    /// Remove only validated saved overrides; permission comes from the guard.
    pub(crate) async fn unset_node_settings(
        &self,
        _: &Connection,
        request: UnsetNodeSettings,
    ) -> Result<NodeSettings, WireError> {
        self.settings
            .unset(&request.keys)
            .await
            .map_err(|error| failure(&error))
    }

    /// Read and validate everything before publishing any runtime setting.
    pub(crate) async fn reload_node_settings(
        &self,
        _: &Connection,
        _: ReloadNodeSettings,
    ) -> Result<ReloadedNodeSettings, WireError> {
        self.settings
            .reload()
            .await
            .map_err(|error| failure(&error))
    }
}

fn failure(error: &SettingsError) -> WireError {
    let code = match error {
        SettingsError::Unknown
        | SettingsError::UnknownDriver(_)
        | SettingsError::Key(_)
        | SettingsError::Reload(vessel_store::ReloadSettingsError::Input(_)) => {
            ErrorCode::InvalidRequest
        }
        SettingsError::NoDirect(_) => ErrorCode::Unsupported,
        SettingsError::Kept(_) | SettingsError::Stopping => ErrorCode::Conflict,
        SettingsError::Write(_)
        | SettingsError::Lost(_)
        | SettingsError::Reload(vessel_store::ReloadSettingsError::Read(_)) => ErrorCode::Internal,
    };
    if matches!(code, ErrorCode::Internal | ErrorCode::Conflict) {
        tracing::warn!(%error, "node settings not kept");
    }
    WireError::new(code, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_settings_task_panic_never_becomes_a_wire_message() {
        let canary = "private-settings-wire-canary";
        let cause = tokio::spawn(async move { panic!("{canary}") })
            .await
            .unwrap_err();
        let error = SettingsError::Lost(cause.into());
        let wire = failure(&error);
        assert!(!format!("{wire:?}").contains(canary));
        assert!(!wire.to_string().contains(canary));
    }
}
