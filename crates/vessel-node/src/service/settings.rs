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
        self.settings.change(request).await.map_err(|error| {
            let code = match error {
                SettingsError::Unknown | SettingsError::UnknownDriver(_) => {
                    ErrorCode::InvalidRequest
                }
                SettingsError::NoDirect(_) => ErrorCode::Unsupported,
                // The node's own file needs a person's hand first.
                SettingsError::Kept(_) | SettingsError::Stopping | SettingsError::Full => {
                    ErrorCode::Conflict
                }
                SettingsError::Write(_)
                | SettingsError::Lost(_)
                | SettingsError::Consumer(_)
                | SettingsError::Recovery(_) => ErrorCode::Internal,
            };
            if matches!(code, ErrorCode::Internal | ErrorCode::Conflict) {
                tracing::warn!(%error, "node settings not kept");
            }
            WireError::new(code, error.to_string())
        })
    }
}
