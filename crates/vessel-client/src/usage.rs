//! Reported usage through the same transport and version guard as sessions.
use crate::{CallError, NodeClient};
use vessel_proto::usage::{QueryUsage, UsageReport};
impl NodeClient {
    /// Read reported tokens and, with operate scope, costs.
    ///
    /// # Errors
    /// `forbidden` for an agent outside its session; `invalid_request`
    /// for a machine/session period mismatch; otherwise see [`CallError`].
    pub async fn usage(&self, request: QueryUsage) -> Result<UsageReport, CallError> {
        self.call(request).await
    }
}
