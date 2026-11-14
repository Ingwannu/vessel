//! Protocol 12's `node.settings` and `node.set_settings` (ADR 0038): the
//! node's settings, kept and applied by `crate::settings`.

use vessel_proto::settings::{GetNodeSettings, NodeSettings, SetNodeSettings};
use vessel_proto::{ErrorCode, WireError};

use super::{Connection, NodeService};
use crate::settings::SettingsError;

impl NodeService {
    /// `node.settings`: the settings in force.
    pub(crate) async fn node_settings(
        &self,
        _: &Connection,
        request: GetNodeSettings,
    ) -> Result<NodeSettings, WireError> {
        let mut current = self.settings.current().await;
        if !request.include_restart_recovery {
            current.restart_recovery = vessel_proto::restart::RestartRecovery::default();
        }
        Ok(current)
    }

    /// `node.set_settings`: change, keep and apply what the request names.
    pub(crate) async fn set_node_settings(
        &self,
        _: &Connection,
        request: SetNodeSettings,
    ) -> Result<NodeSettings, WireError> {
        self.settings.change(request).await.map_err(|error| {
            let code = match error {
                SettingsError::Unknown | SettingsError::UnknownDriver(_) => {
                    ErrorCode::InvalidRequest
                }
                SettingsError::NoDirect(_) => ErrorCode::Unsupported,
                SettingsError::Full | SettingsError::Closed => ErrorCode::Unavailable,
                SettingsError::Write(_)
                | SettingsError::Lost(_)
                | SettingsError::Reply(_)
                | SettingsError::Cleanup(_)
                | SettingsError::Apply(_) => ErrorCode::Internal,
            };
            if code == ErrorCode::Internal {
                tracing::warn!(%error, "node settings change did not fully apply");
            }
            WireError::new(code, error.to_string())
        })
    }
}
