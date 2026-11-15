//! Inspect one authenticated connection; no local state or inferred fallback.
use crate::NodeClient;
use vessel_proto::{DeviceSelf, GetDeviceSelf};
use vessel_rpc::CallError;

impl NodeClient {
    /// This connection's own effective scopes, without requiring Read.
    /// Resource ownership and agent limits remain separate checks.
    ///
    /// # Errors
    /// An older peer is `NotInProtocol`; retain unknown authority rather
    /// than guessing full or read-only access. Authentication can fail.
    pub async fn device_self(&self) -> Result<DeviceSelf, CallError> {
        self.call(GetDeviceSelf {}).await
    }
}
