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

use vessel_proto::limits::{ExternalLookup, GetUsageLimits, SetUsageLookup, UsageLimits};
impl NodeClient {
    /// Latest saved-login facts; operate is required for a requested refresh.
    /// # Errors
    /// Version, scope, unknown profile or node admission refusals.
    pub async fn usage_limits(&self, request: GetUsageLimits) -> Result<UsageLimits, CallError> {
        self.call(request).await
    }
    /// Save this machine's credential-use choice; admin permission required.
    /// # Errors
    /// Version, scope, storage or node admission refusals.
    pub async fn usage_lookup(&self, request: SetUsageLookup) -> Result<ExternalLookup, CallError> {
        self.call(request).await
    }
}
