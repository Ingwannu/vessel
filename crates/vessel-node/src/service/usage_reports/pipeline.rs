//! Captured binding and actual actor read stay one data path, including A→B→A edits.
use crate::{
    limits::{Limits, policy::Observation},
    usage_reports::{self, Binding, Registry},
};
use std::future::Future;
use vessel_proto::{
    HarnessRef, WireError,
    limits::{LimitRefresh, LimitsScope},
    usage::CostVisibility,
    usage_reports::SourceReportEntry,
};
/// Metadata may be refreshed, but the read cannot resolve configuration again.
pub(super) trait Bindings: Sync {
    fn capture(
        &self,
        scope: LimitsScope,
    ) -> impl Future<Output = Result<(Binding, HarnessRef), WireError>> + Send;
    fn registry(&self) -> &Registry;
    fn limits(&self) -> &Limits;
    fn visibility(&self) -> CostVisibility;
}
pub(super) async fn saved_report(
    reader: &impl Bindings,
    scope: LimitsScope,
    refresh: LimitRefresh,
    policy: &Observation,
) -> Result<SourceReportEntry, WireError> {
    let (binding, harness) = reader.capture(scope).await?;
    let source = usage_reports::source(&harness).ok_or_else(|| {
        WireError::new(
            vessel_proto::ErrorCode::Unsupported,
            "this saved login has no usage report source",
        )
    })?;
    let id = reader
        .registry()
        .identify(
            source.id().map_err(|_invalid_source| super::changed())?,
            binding.clone(),
            policy,
        )
        .await?;
    // Never call a selector-based service here: it could choose B between
    // capture A and the after-check returning to A. This read receives A itself.
    let limits = reader
        .limits()
        .read_observed(
            binding.account.clone(),
            binding.proof.clone(),
            refresh,
            policy,
        )
        .await?;
    let (after, _) = reader.capture(binding.scope.clone()).await?;
    if after != binding || limits.account != binding.account || !policy.still_current() {
        return Err(super::changed());
    }
    let label = limits
        .account
        .profile
        .as_ref()
        .map(vessel_proto::ProfileName::as_str)
        .map(str::to_owned);
    let entry = usage_reports::entry(id, label.as_deref(), limits).map_err(|_projection| {
        WireError::new(
            vessel_proto::ErrorCode::Internal,
            "the complete saved account report could not be represented safely",
        )
    })?;
    Ok(entry.for_visibility(reader.visibility()))
}
#[cfg(test)]
mod tests;
