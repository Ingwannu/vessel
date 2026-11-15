//! Settings project before secret I/O; the shared file/lock owner applies changes.
use super::{Connection, NodeService};
use crate::settings::SettingsError;
use vessel_proto::settings::{GetNodeSettings, NodeSettings, SetNodeSettings};
use vessel_proto::{
    ErrorCode, ErrorDetail, WireError,
    dictation::{DictationReason, SINCE},
};

impl NodeService {
    /// Unassigned and older reads skip presence lookup entirely.
    pub(crate) async fn node_settings(
        &self,
        connection: &Connection,
        _: GetNodeSettings,
    ) -> Result<NodeSettings, WireError> {
        if SINCE.is_some_and(|required| connection.protocol() >= required) {
            self.settings
                .current_dictation(self.dictation.reader())
                .await
                .map_err(|error| refusal(&error))
        } else {
            Ok(self.settings.current().await)
        }
    }
    /// Validate and persist one whole candidate before applying anything.
    pub(crate) async fn set_node_settings(
        &self,
        connection: &Connection,
        request: SetNodeSettings,
    ) -> Result<NodeSettings, WireError> {
        if SINCE.is_some_and(|required| connection.protocol() >= required) {
            self.settings
                .change_dictation(request, self.dictation.reader())
                .await
        } else {
            self.settings.change(request).await
        }
        .map_err(|error| refusal(&error))
    }
}
fn refusal(error: &SettingsError) -> WireError {
    let code = match error {
        SettingsError::Unknown | SettingsError::UnknownDriver(_) | SettingsError::Dictation(_) => {
            ErrorCode::InvalidRequest
        }
        SettingsError::NoDirect(_) | SettingsError::DictationUnavailable => ErrorCode::Unsupported,
        SettingsError::Kept(_) | SettingsError::DictationKeyAbsent => ErrorCode::Conflict,
        SettingsError::Write(_) | SettingsError::Lost(_) | SettingsError::DictationSecret(_) => {
            ErrorCode::Internal
        }
    };
    if matches!(code, ErrorCode::Internal | ErrorCode::Conflict) {
        // Every private secret cause stays behind its fixed outer diagnostic.
        tracing::warn!(%error, "node settings not kept");
    }
    let mut reply = WireError::new(code, error.to_string());
    if matches!(error, SettingsError::DictationKeyAbsent) {
        reply.detail = Some(ErrorDetail::Dictation {
            reason: DictationReason::KeyAbsent,
        });
    }
    reply
}

#[cfg(test)]
mod dictation_tests;
