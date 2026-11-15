//! Recovery policy settlement inside the one canonical settings operation.
//!
//! [Decision Log]
//! - Purpose: save acknowledgement loss must not abandon a durable Off or
//!   allow a later On to overtake pending intent cleanup.
//! - Constraint: the settings owner already retains its opaque saved guard,
//!   physical writer and accepted operation; this hook owns none of those.
//! - Choice: compose the existing Policy arbiter and Store operation directly.
//!   A callback, second queue or parser would split ownership and was rejected.
//! - Cost: failed SQL apply remains explicit debt and can hold settings drain.
use crate::recovery::policy::Policy;
use std::sync::Arc;
use vessel_proto::restart::RestartRecovery;
use vessel_store::{Store, StoreError};

#[derive(Debug, thiserror::Error)]
pub(crate) enum RecoveryApplyError {
    #[error(
        "startup recovery policy was saved but could not be applied; retry this settings change: {0}"
    )]
    Apply(#[source] StoreError),
    #[error(
        "startup recovery is off but pending intents could not be cleared; retry this settings change: {0}"
    )]
    Cleanup(#[source] StoreError),
}

/// Concrete field logic only. The settings owner retains its saved guard
/// through every method, including unchanged-value settlement.
#[derive(Debug)]
pub(crate) struct RecoveryApply {
    policy: Arc<Policy>,
    store: Store,
}
impl RecoveryApply {
    pub(crate) fn new(policy: Arc<Policy>, store: Store) -> Self {
        Self { policy, store }
    }
    /// Seal before settings drain, under the same mutex as final effect enqueue.
    pub(crate) fn seal_effect_admission(&self) {
        self.policy.seal_effect_admission();
    }
    /// A failed saved Off must be paid before any later mutation, including On.
    pub(crate) async fn settle_before(&self) -> Result<(), RecoveryApplyError> {
        if self.policy.needs_cleanup() {
            self.store
                .set_restart_policy(false)
                .await
                .map_err(RecoveryApplyError::Cleanup)?;
            self.policy.cleaned();
        }
        Ok(())
    }
    pub(crate) fn begin_write(&self, next: &RestartRecovery) {
        self.policy.begin_write(next);
    }
    pub(crate) fn write_failed(&self) {
        self.policy.write_failed();
    }
    /// Called after actual file persistence and before awaiting SQL settlement.
    pub(crate) fn saved(&self, next: RestartRecovery) {
        self.policy.apply(next);
    }
    /// This path also serves next==saved; failed apply cannot become a false
    /// successful no-op. The caller owns the actual await despite reply loss.
    pub(crate) async fn settle_saved(
        &self,
        current: &RestartRecovery,
    ) -> Result<(), RecoveryApplyError> {
        if self.policy.needs_sync() {
            self.store
                .set_restart_policy(current.enabled)
                .await
                .map_err(|error| {
                    if current.enabled {
                        RecoveryApplyError::Apply(error)
                    } else {
                        RecoveryApplyError::Cleanup(error)
                    }
                })?;
            self.policy.cleaned();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
