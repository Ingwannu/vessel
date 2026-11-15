//! Opaque ids are scoped to exact resolved bindings and the existing policy generation.
use crate::limits::policy::Observation;
use tokio::sync::Mutex;
use vessel_harness::LimitBinding;
use vessel_proto::{
    ErrorCode, WireError,
    limits::{LimitFailure, LimitsScope},
    usage_reports::{MAX_USAGE_REPORTS, UsageReportId, UsageSourceId},
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Binding {
    pub(crate) account: vessel_proto::limits::LimitAccount,
    pub(crate) scope: LimitsScope,
    pub(crate) proof: Result<LimitBinding, LimitFailure>,
}
#[derive(Debug, Clone)]
struct Known {
    id: UsageReportId,
    binding: Binding,
    policy: Observation,
}
/// This holds sanitized configuration references, never a credential or provider body.
#[derive(Debug, Default)]
pub(crate) struct Registry(Mutex<Vec<Known>>);
impl Registry {
    pub(crate) async fn identify(
        &self,
        source: UsageSourceId,
        binding: Binding,
        policy: &Observation,
    ) -> Result<UsageReportId, WireError> {
        let mut known = self.0.lock().await;
        known.retain(|entry| entry.policy.still_current());
        if !policy.still_current() {
            return Err(changed());
        }
        if let Some(entry) = known
            .iter()
            .find(|entry| entry.binding == binding && entry.id.source() == &source)
        {
            return Ok(entry.id.clone());
        }
        if known.len() >= MAX_USAGE_REPORTS {
            return Err(WireError::new(
                ErrorCode::Unavailable,
                "usage report registry is full; no account was omitted",
            ));
        }
        let key = uuid::Uuid::new_v4()
            .to_string()
            .parse()
            .map_err(|_invalid_identity| changed())?;
        let id = UsageReportId::new(source, key);
        known.push(Known {
            id: id.clone(),
            binding,
            policy: policy.clone(),
        });
        Ok(id)
    }
    pub(crate) async fn resolve(&self, id: &UsageReportId) -> Option<Binding> {
        let mut known = self.0.lock().await;
        known.retain(|entry| entry.policy.still_current());
        known
            .iter()
            .find(|entry| &entry.id == id)
            .map(|entry| entry.binding.clone())
    }
    pub(crate) async fn forget_cancelled(&self) {
        self.0
            .lock()
            .await
            .retain(|entry| entry.policy.still_current());
    }
    #[cfg(test)]
    pub(crate) async fn hold_cleanup(
        &self,
        entered: tokio::sync::oneshot::Sender<()>,
        released: tokio::sync::oneshot::Receiver<()>,
    ) {
        // The test deliberately holds this Tokio mutex across the barrier so
        // the real service setter cannot reach its caller-owned cleanup.
        let _held = self.0.lock().await;
        let _closed = entered.send(());
        let _closed = released.await;
    }
}
fn changed() -> WireError {
    WireError::new(
        ErrorCode::Conflict,
        "usage report bindings changed; read reports again",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::sync::CancellationToken;
    use vessel_proto::limits::LimitAccount;
    fn binding(name: &str) -> Binding {
        Binding {
            account: LimitAccount {
                harness: "codex".parse().unwrap(),
                profile: Some(name.parse().unwrap()),
            },
            scope: LimitsScope::Account {
                account: LimitAccount {
                    harness: "codex".parse().unwrap(),
                    profile: Some(name.parse().unwrap()),
                },
            },
            proof: Err(LimitFailure::DifferentRoute),
        }
    }
    #[tokio::test]
    async fn identical_bindings_reuse_ids_but_different_scopes_do_not() {
        let registry = Registry::default();
        let policy = Observation::fixture(CancellationToken::new());
        let source = "saved-codex".parse().unwrap();
        let first = registry
            .identify(source, binding("first"), &policy)
            .await
            .unwrap();
        assert_eq!(
            registry
                .identify("saved-codex".parse().unwrap(), binding("first"), &policy)
                .await
                .unwrap(),
            first
        );
        let second = registry
            .identify("saved-codex".parse().unwrap(), binding("second"), &policy)
            .await
            .unwrap();
        assert_ne!(first, second);
        assert_eq!(registry.resolve(&first).await, Some(binding("first")));
        assert!(!first.to_string().contains("first"));
    }
    #[tokio::test]
    async fn off_revokes_discovery_resolution_and_late_registration() {
        let registry = Registry::default();
        let generation = CancellationToken::new();
        let before = Observation::fixture(generation.child_token());
        let id = registry
            .identify("saved-codex".parse().unwrap(), binding("first"), &before)
            .await
            .unwrap();
        generation.cancel();
        assert!(registry.resolve(&id).await.is_none());
        assert!(
            registry
                .identify("saved-codex".parse().unwrap(), binding("late"), &before)
                .await
                .is_err()
        );
        let after = Observation::fixture(CancellationToken::new());
        let new = registry
            .identify("saved-codex".parse().unwrap(), binding("first"), &after)
            .await
            .unwrap();
        assert_ne!(id, new);
        registry.forget_cancelled().await;
        assert!(registry.resolve(&new).await.is_some());
    }
    #[tokio::test]
    async fn a_full_registry_refuses_growth_without_evicting_an_existing_id() {
        let registry = Registry::default();
        let policy = Observation::fixture(CancellationToken::new());
        let mut first = None;
        for index in 0..MAX_USAGE_REPORTS {
            let id = registry
                .identify(
                    "saved-codex".parse().unwrap(),
                    binding(&format!("account-{index}")),
                    &policy,
                )
                .await
                .unwrap();
            if first.is_none() {
                first = Some(id);
            }
        }
        assert!(
            registry
                .identify("saved-codex".parse().unwrap(), binding("overflow"), &policy)
                .await
                .is_err()
        );
        let first = first.unwrap();
        assert_eq!(
            registry
                .identify(
                    "saved-codex".parse().unwrap(),
                    binding("account-0"),
                    &policy
                )
                .await
                .unwrap(),
            first
        );
        assert!(registry.resolve(&first).await.is_some());
    }
}
