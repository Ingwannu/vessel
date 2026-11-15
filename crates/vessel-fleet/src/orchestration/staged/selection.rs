//! Serialized selection save: confirm a fresh plan, capture, converge, persist last.

use vessel_proto::skills::{SkillNames, SkillOperation, SkillSelection, SkillsSaved};

use super::{SkillRoot, SkillService, deletion};
use crate::orchestration::{SelectionOwnerError, SkillError};

const MAX_SAVE_ATTEMPTS: usize = 3;

/// Persistence errors retain a separate rollback failure, including its backups.
#[derive(Debug, thiserror::Error)]
pub enum SelectionError<E: std::error::Error + 'static> {
    /// Files could not be captured, changed or recovered safely.
    #[error(transparent)]
    Skill(#[from] SkillError),
    /// No choice was acknowledged; the rollback result remains explicit.
    #[error("skill selection persistence failed; rollback: {rollback:?}")]
    Persist {
        /// Error returned by the owning node's store.
        source: SelectionOwnerError<E>,
        /// None means all this transaction's changes were rolled back.
        rollback: Option<SkillError>,
    },
    /// An apply error and a rollback conflict must both remain visible.
    #[error("skill selection apply failed: {source}; rollback: {rollback}")]
    Rollback {
        /// Original application failure.
        source: SkillError,
        /// Recovery failed without deleting its evidence.
        rollback: SkillError,
    },
}

impl SkillService {
    pub(super) fn recover(
        &mut self,
        state: &SkillRoot,
        durable: Option<&SkillSelection>,
    ) -> Result<(), SkillError> {
        deletion::recover(&self.homes, state, durable)
    }

    pub(super) fn save<E: std::error::Error + 'static>(
        &mut self,
        state: &SkillRoot,
        previous: Option<SkillSelection>,
        next: &SkillSelection,
        confirmed: &SkillNames,
        persist: impl FnOnce(&SkillSelection) -> Result<(), E>,
    ) -> Result<SkillsSaved, SelectionError<E>> {
        self.recover(state, previous.as_ref())?;
        let mut plan = self.status(next)?;
        for _attempt in 0..MAX_SAVE_ATTEMPTS {
            let removals = SkillNames::new(
                plan.ops
                    .iter()
                    .filter_map(|op| {
                        if let SkillOperation::Delete { name } = op {
                            Some(name.clone())
                        } else {
                            None
                        }
                    })
                    .collect(),
            )
            .map_err(SkillError::from)?;
            if removals
                .as_slice()
                .iter()
                .any(|name| !confirmed.contains(name))
            {
                return Ok(SkillsSaved {
                    snapshot: self.status(&previous.unwrap_or_default())?,
                    confirmation_required: Some(removals),
                });
            }
            let mut transaction = deletion::DeletionTransaction::begin(
                &self.homes,
                state,
                previous.clone(),
                next.clone(),
                &plan,
            )?;
            transaction.capture_patches(self, &plan)?;
            let applied = transaction
                .apply()
                .and_then(|()| self.apply_updates(&plan))
                .and_then(|()| self.status(next));
            let snapshot = match applied {
                Ok(snapshot) => snapshot,
                Err(source) => {
                    return Err(match transaction.rollback() {
                        Ok(()) => SelectionError::Skill(source),
                        Err(rollback) => SelectionError::Rollback { source, rollback },
                    });
                }
            };
            if snapshot.ops.is_empty() {
                if let Err(source) = persist(next) {
                    return Err(SelectionError::Persist {
                        source: source.into(),
                        rollback: transaction.rollback().err(),
                    });
                }
                // Files and the preference are already durable. A retained journal
                // retries cleanup on the next command; a save must not report failure.
                if let Err(error) = transaction.commit() {
                    tracing::warn!(%error, "committed skill save has pending journal cleanup");
                }
                return Ok(SkillsSaved {
                    snapshot,
                    confirmation_required: None,
                });
            }
            transaction.rollback()?;
            plan = self.status(next)?;
        }
        Err(SkillError::Unstable.into())
    }
}

#[cfg(test)]
mod tests;
