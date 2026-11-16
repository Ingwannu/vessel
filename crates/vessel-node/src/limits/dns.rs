//! The usage client's one tracked name-resolution scope (ADR 0104, third consumer).
//!
//! [Decision Log] (2026-11-16)
//! - Purpose: the persistent provider client behind `LimitContext` is shared
//!   by saved-sign-in reads (`usage.limits`) and additional account sources
//!   (`usage.reports`). Its lookups must stay owned until native completion
//!   and the node may not report a normal stop before they end.
//! - Constraint: reqwest's `Resolve` call carries no request identity, so a
//!   lookup cannot be charged to the read that caused it.
//! - Alternatives: one scope per read (needs a request identity reqwest does
//!   not give, or a task-local association the contract forbids); pinning
//!   the first initialiser's read permit for the client lifetime (makes one
//!   read look endless and still proves nothing about later lookups).
//! - Choice (coordinator ruling, ADR 0104 usage supplement): ONE scope for the
//!   one client lifetime, sized from the existing read bound on the node-root
//!   budget, closed and drained only after both source owners and every job
//!   they started have ended. The client is never replaced inside a context,
//!   so node stop is the only retirement.
//! - Cost: a stuck OS resolver keeps node stop waiting (no deadline bounds
//!   native DNS). The per-read admission stays a separate logical bound.
use std::sync::Arc;
use tokio_util::task::TaskTracker;
use vessel_harness::LimitContext;
use vessel_relay::resolution::{ResolutionBudget, ResolutionQuota, ResolutionScope};

/// Permits-only consumer hold. It captures nothing, so physical leases keep
/// no configuration, credential, request, read permit or registry alive.
pub(super) struct UsageClient;

/// The context for both source owners and the scope its client resolves
/// through. Without a scope the context fails every live read closed.
pub(super) fn context(
    budget: &ResolutionBudget,
) -> (LimitContext, Option<ResolutionScope<UsageClient>>) {
    match ResolutionQuota::new(LimitContext::READ_SLOTS) {
        Ok(quota) => {
            let scope = budget.scope(quota, Arc::new(UsageClient));
            let resolver: Arc<dyn reqwest::dns::Resolve> =
                Arc::new(crate::http_resolution::HttpResolver::new(scope.handle()));
            (LimitContext::resolving(resolver), Some(scope))
        }
        Err(error) => {
            tracing::error!(%error, "usage name resolution has no valid quota; live usage reads stay off");
            (LimitContext::new(), None)
        }
    }
}

/// Run on the node's task tracker, so node stop waits for it. `owners` holds
/// exactly the two source owners; each joins its own jobs and drains the
/// read admission before it ends. Only then is admission closed and every
/// physical lookup joined. A failed completion owner is reported, never
/// turned into a normal stop.
pub(super) async fn retire(owners: TaskTracker, scope: Option<ResolutionScope<UsageClient>>) {
    let _first_close = owners.close();
    owners.wait().await;
    if let Some(scope) = scope
        && let Err(error) = scope.drain().await
    {
        tracing::error!(%error, "usage name resolution did not drain normally");
    }
}

#[cfg(test)]
mod tests;
