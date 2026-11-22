//! Run identity, recovery generations and diagnostics for the node's owner.
//! Storage/authority stay in the node; these calls use the existing actor.
use super::SessionHandle;
use crate::error::SessionError;
use crate::moving::Control;
use std::sync::Arc;

impl SessionHandle {
    /// Diagnostic envelope bound, distinct from the node's failure summary.
    const RECOVERY_NOTICE_CHARS: usize = 768;

    /// Whether both handles refer to this exact run, rather than just its id.
    #[must_use]
    pub fn same_run(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// Current stop generation, read at an input's admission.
    #[must_use]
    pub fn restart_generation(&self) -> vessel_proto::restart::RestartGeneration {
        self.0.journal.generation()
    }

    /// Restore a generation before the gated actor publishes anything.
    ///
    /// # Errors
    /// This run already published a record.
    pub fn initialise_restart_generation(
        &self,
        generation: vessel_proto::restart::RestartGeneration,
    ) -> Result<(), crate::JournalError> {
        self.0.journal.initialise(generation)
    }

    /// Record a bounded node diagnostic through this run's ordinary actor.
    ///
    /// # Errors
    /// The session ended or moved before the diagnostic could be published.
    pub async fn recovery_notice(&self, line: String) -> Result<(), SessionError> {
        let route = self.structured("a conversation")?;
        // Diagnostics do not carry arbitrarily large driver errors into a
        // one-slot actor queue, and cannot inject terminal control sequences.
        let line = line
            .chars()
            .filter(|character| !character.is_control())
            .take(Self::RECOVERY_NOTICE_CHARS)
            .collect();
        self.order(route, |reply| Control::Notice { line, reply })
            .await
    }

    /// Saved-sign-in association of the actual launch, without credentials.
    #[must_use]
    pub fn limit_association(&self) -> crate::LimitAssociation {
        self.0.limits.borrow().clone()
    }
}
