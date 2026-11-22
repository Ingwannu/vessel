//! Explicit recovery capability reads; absent legacy fields never mean confirmed Off.
use crate::{CallError, NodeClient};
use vessel_proto::restart::{RestartRecovery, SetRestartRecovery};
use vessel_proto::settings::{GetNodeSettings, SetNodeSettings};
impl NodeClient {
    /// Read recovery only when its feature requirement is assigned and supported.
    ///
    /// # Errors
    /// Canonical unassigned/older-peer admission, scope or transport failure.
    pub async fn restart_recovery(&self) -> Result<RestartRecovery, CallError> {
        Ok(self
            .call(GetNodeSettings {
                include_restart_recovery: true,
            })
            .await?
            .restart_recovery)
    }
    /// Save a partial recovery change without changing parking or drivers.
    ///
    /// # Errors
    /// Admission, admin scope, save/apply or transport failure. A lost reply
    /// does not mean the admitted save was cancelled; do not automatically retry.
    pub async fn set_restart_recovery(
        &self,
        change: SetRestartRecovery,
    ) -> Result<RestartRecovery, CallError> {
        Ok(self
            .call(SetNodeSettings {
                restart_recovery: Some(change),
                ..SetNodeSettings::default()
            })
            .await?
            .restart_recovery)
    }
}
