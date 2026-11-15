//! The existing Read/Admin settings methods; relay failures cross as closed codes.
use super::{Connection, NodeService};
use crate::settings::SettingsError;
use vessel_proto::relay_settings::RelaySettingsError;
use vessel_proto::settings::{GetNodeSettings, NodeSettings, SetNodeSettings};
use vessel_proto::{ErrorCode, ErrorDetail, WireError};

impl NodeService {
    /// `node.settings`: the settings in force.
    pub(crate) async fn node_settings(
        &self,
        _: &Connection,
        _: GetNodeSettings,
    ) -> Result<NodeSettings, WireError> {
        Ok(self.settings.current().await)
    }

    /// The existing guard validates the full method before this admission path.
    pub(crate) async fn set_node_settings(
        &self,
        _: &Connection,
        request: SetNodeSettings,
    ) -> Result<NodeSettings, WireError> {
        let kind = if request.relay.is_some() {
            ChangeKind::Relay
        } else {
            ChangeKind::Ordinary
        };
        self.settings
            .change(request)
            .await
            .map_err(|error| failure(error, kind))
    }
}

/// Legacy unrelated messages stay unchanged. Relay requests cannot publish a
/// storage path, private configuration or a lower-layer diagnostic via Display.
fn failure(error: SettingsError, kind: ChangeKind) -> WireError {
    let code = match &error {
        SettingsError::Unknown | SettingsError::UnknownDriver(_) => ErrorCode::InvalidRequest,
        SettingsError::NoDirect(_) => ErrorCode::Unsupported,
        SettingsError::Kept(_) => ErrorCode::Conflict,
        SettingsError::Write(_) | SettingsError::Lost(_) => ErrorCode::Internal,
        SettingsError::Relay(reason) => match reason {
            RelaySettingsError::NotConfigured
            | RelaySettingsError::Stopping
            | RelaySettingsError::Busy => ErrorCode::Conflict,
            RelaySettingsError::Uncertain => ErrorCode::Unavailable,
            RelaySettingsError::Unknown => ErrorCode::InvalidRequest,
        },
    };
    if let SettingsError::Relay(reason) = error {
        // Its type has only fixed messages and closed variants.
        return relay_failure(code, reason);
    }
    if matches!(kind, ChangeKind::Relay) {
        tracing::warn!(?code, "relay settings change was not kept");
        let message = match error {
            SettingsError::Unknown | SettingsError::UnknownDriver(_) => {
                "the settings request contains an unknown choice; refresh settings before changing them"
            }
            SettingsError::NoDirect(_) => {
                "this node does not offer the requested agent driver; choose a supported driver"
            }
            SettingsError::Kept(_) => {
                "the settings file needs repair; ask the operator to repair it before changing settings"
            }
            SettingsError::Write(_) | SettingsError::Lost(_) => {
                "the settings could not be kept; inspect current settings before trying another change"
            }
            SettingsError::Relay(reason) => return relay_failure(code, reason),
        };
        return WireError::new(code, message);
    }
    if matches!(code, ErrorCode::Internal | ErrorCode::Conflict) {
        tracing::warn!(%error, "node settings not kept");
    }
    WireError::new(code, error.to_string())
}

#[derive(Clone, Copy)]
enum ChangeKind {
    Ordinary,
    Relay,
}

fn relay_failure(code: ErrorCode, reason: RelaySettingsError) -> WireError {
    let mut error = WireError::new(code, reason.to_string());
    error.detail = Some(ErrorDetail::RelaySettings { reason });
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn private_storage_diagnostics_never_cross_the_relay_boundary() {
        let private = "synthetic-token vessel://synthetic-invitation /home/synthetic/private";
        let error = failure(SettingsError::Kept(Arc::from(private)), ChangeKind::Relay);
        assert_eq!(error.code, ErrorCode::Conflict);
        assert_eq!(
            error.message,
            "the settings file needs repair; ask the operator to repair it before changing settings"
        );
        let encoded = serde_json::to_string(&error).unwrap();
        for canary in [
            "synthetic-token",
            "vessel://synthetic-invitation",
            "/home/synthetic/private",
        ] {
            assert!(!encoded.contains(canary));
            assert!(!format!("{error:?}").contains(canary));
        }
    }
    #[test]
    fn relay_refusals_have_exact_codes_and_closed_details() {
        for (reason, code) in [
            (RelaySettingsError::NotConfigured, ErrorCode::Conflict),
            (RelaySettingsError::Stopping, ErrorCode::Conflict),
            (RelaySettingsError::Busy, ErrorCode::Conflict),
            (RelaySettingsError::Uncertain, ErrorCode::Unavailable),
            (RelaySettingsError::Unknown, ErrorCode::InvalidRequest),
        ] {
            let failure = failure(SettingsError::Relay(reason), ChangeKind::Relay);
            assert_eq!(failure.code, code);
            assert_eq!(failure.detail, Some(ErrorDetail::RelaySettings { reason }));
            assert_eq!(failure.message, reason.to_string());
        }
    }
}
