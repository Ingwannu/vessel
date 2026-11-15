//! Typed provider calls. Pending feature versions are refused by the shared
//! connection before sending; the client never invents a protocol number.

use crate::NodeClient;
use vessel_proto::providers::{
    GetProviderDiagnostic, GetProviders, ProviderDiagnostic, ProviderSnapshotEntry,
    SetProviderEnabled,
};
use vessel_rpc::CallError;

impl NodeClient {
    /// Read metadata or explicitly discover providers under the requested scope.
    ///
    /// # Errors
    /// Unassigned/older protocol, scope refusal, bounded discovery or connection
    /// failure. Cached reads never upgrade to a process probe on a miss.
    pub async fn providers(
        &self,
        request: GetProviders,
    ) -> Result<Vec<ProviderSnapshotEntry>, CallError> {
        self.call(request).await
    }

    /// Read the same safe cache projection consumed by provider snapshots.
    ///
    /// # Errors
    /// As providers; active discovery additionally needs Operate and a person.
    pub async fn provider_diagnostic(
        &self,
        request: GetProviderDiagnostic,
    ) -> Result<ProviderDiagnostic, CallError> {
        self.call(request).await
    }

    /// Preserve every launch override while changing participation as an Admin.
    ///
    /// # Errors
    /// Protocol/authority refusal or a failed durable write. The old file and
    /// applied registry remain on validation/write failure.
    pub async fn set_provider_enabled(
        &self,
        request: SetProviderEnabled,
    ) -> Result<ProviderSnapshotEntry, CallError> {
        self.call(request).await
    }
}
