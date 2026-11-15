//! Deferred setup choices are a batch, not immediate host-setting mutations.
//!
//! The caller executes only the returned typed operations. All offered effects
//! must succeed for the captured visit; partial success is retained on retry.

use super::Visit;
use std::collections::{BTreeMap, BTreeSet};

/// Independent integration capabilities; unsupported ones have no operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Integration {
    /// Inject orchestration tools into agents.
    Tools,
    /// Reconcile or uninstall the selected skill set.
    Skills,
    /// Install or remove the desktop-owned CLI entry.
    Cli,
}

/// A choice cannot be confused with a readiness or success flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Offered integrations start selected, matching Paseo's `ALL_ON`.
    On,
    /// The person explicitly disabled it.
    Off,
}

/// The controller executes this operation and returns its exact identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation {
    /// Which integration changes.
    pub integration: Integration,
    /// Desired value, captured before any I/O.
    pub choice: Choice,
    /// A result from an old step can settle only its own batch.
    pub visit: Visit,
    attempt: Attempt,
}

/// A retry on the same visit cannot accept duplicate replies from an older batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Attempt(u64);

/// A failed integration remains visible and retryable; never infer from text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The requested value is now in effect.
    Applied,
    /// Its owner has already retained a typed, user-visible failure.
    Refused,
}

/// Completion is distinct from a partially successful or superseded batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Settlement {
    /// Other effects or a failed effect still prevent advancement.
    Waiting,
    /// Every captured operation succeeded for this visit.
    Complete(Visit),
    /// No active batch owns this result.
    Stale,
}

/// Choice edits are refused while a captured batch is applying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ApplyError {
    /// Repeated Continue must not duplicate installs.
    #[error("setup integrations are still applying")]
    Applying,
    /// Reusing a wrapped batch identity could acknowledge old work.
    #[error("setup integration attempts exhausted")]
    AttemptsExhausted,
}

/// Owns deferred choices and the current visit's acknowledgements.
#[derive(Debug, Default)]
pub struct Orchestration {
    choices: BTreeMap<Integration, Choice>,
    active: Option<Visit>,
    pending: BTreeMap<Integration, Choice>,
    failures: BTreeSet<Integration>,
    attempts: u64,
    attempt: Option<Attempt>,
}

impl Orchestration {
    /// Default-on choices are read without causing any writes.
    #[must_use]
    pub fn choice(&self, integration: Integration) -> Choice {
        self.choices
            .get(&integration)
            .copied()
            .unwrap_or(Choice::On)
    }

    /// Changing a switch alone never returns an operation.
    ///
    /// # Errors
    /// Another Continue batch is still applying.
    pub fn choose(&mut self, integration: Integration, choice: Choice) -> Result<(), ApplyError> {
        if self.active.is_some() {
            return Err(ApplyError::Applying);
        }
        let _previous = self.choices.insert(integration, choice);
        Ok(())
    }

    /// Continue captures only offered values that are not already known equal.
    ///
    /// # Errors
    /// A batch is already applying; repeated clicks must not duplicate installs.
    pub fn begin(
        &mut self,
        visit: Visit,
        offered: &BTreeMap<Integration, Option<Choice>>,
    ) -> Result<Vec<Operation>, ApplyError> {
        if self.active.is_some() {
            return Err(ApplyError::Applying);
        }
        let attempts = self
            .attempts
            .checked_add(1)
            .ok_or(ApplyError::AttemptsExhausted)?;
        let attempt = Attempt(attempts);
        self.attempts = attempts;
        self.attempt = Some(attempt);
        self.failures.clear();
        let operations: Vec<_> = offered
            .iter()
            .filter_map(|(integration, current)| {
                let choice = self.choice(*integration);
                (*current != Some(choice)).then_some(Operation {
                    integration: *integration,
                    choice,
                    visit,
                    attempt,
                })
            })
            .collect();
        self.pending = operations
            .iter()
            .map(|operation| (operation.integration, operation.choice))
            .collect();
        self.active = (!operations.is_empty()).then_some(visit);
        Ok(operations)
    }

    /// Superseding a visit drops its local bookkeeping, never rolls back effects.
    pub fn abandon(&mut self) {
        self.active = None;
        self.attempt = None;
        self.pending.clear();
        self.failures.clear();
    }

    /// Late and duplicated replies cannot acknowledge a different operation.
    pub fn settle(&mut self, operation: Operation, outcome: Outcome) -> Settlement {
        if self.active != Some(operation.visit)
            || self.attempt != Some(operation.attempt)
            || self.pending.get(&operation.integration) != Some(&operation.choice)
        {
            return Settlement::Stale;
        }
        let _removed = self.pending.remove(&operation.integration);
        match outcome {
            Outcome::Applied => {}
            Outcome::Refused => {
                let _new = self.failures.insert(operation.integration);
            }
        }
        if self.pending.is_empty() {
            self.active = None;
            if self.failures.is_empty() {
                return Settlement::Complete(operation.visit);
            }
        }
        Settlement::Waiting
    }

    /// Failed effects can be shown beside their own switches and retried.
    #[must_use]
    pub const fn failures(&self) -> &BTreeSet<Integration> {
        &self.failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_are_deferred_supported_and_acknowledged_as_one_batch() {
        let mut model = Orchestration::default();
        assert_eq!(model.choice(Integration::Skills), Choice::On);
        model.choose(Integration::Tools, Choice::Off).unwrap();
        let offered = BTreeMap::from([
            (Integration::Tools, Some(Choice::On)),
            (Integration::Cli, None),
        ]);
        let operations = model.begin(Visit(1), &offered).unwrap();
        assert_eq!(operations.len(), 2);
        assert_eq!(model.begin(Visit(1), &offered), Err(ApplyError::Applying));
        assert_eq!(
            model.choose(Integration::Cli, Choice::Off),
            Err(ApplyError::Applying)
        );
        assert_eq!(
            model.settle(operations[0], Outcome::Applied),
            Settlement::Waiting
        );
        assert_eq!(
            model.settle(operations[1], Outcome::Applied),
            Settlement::Complete(Visit(1))
        );
        assert_eq!(
            model.settle(operations[1], Outcome::Applied),
            Settlement::Stale
        );
    }

    #[test]
    fn failed_or_superseded_work_never_advances_setup() {
        let mut model = Orchestration::default();
        let offered = BTreeMap::from([(Integration::Tools, None), (Integration::Skills, None)]);
        let old = model.begin(Visit(1), &offered).unwrap();
        assert_eq!(model.settle(old[0], Outcome::Refused), Settlement::Waiting);
        assert_eq!(model.settle(old[1], Outcome::Applied), Settlement::Waiting);
        assert_eq!(model.failures(), &BTreeSet::from([Integration::Tools]));
        let new = model.begin(Visit(1), &offered).unwrap();
        assert_eq!(model.settle(old[0], Outcome::Applied), Settlement::Stale);
        model.abandon();
        assert_eq!(model.settle(new[0], Outcome::Applied), Settlement::Stale);
    }
}
