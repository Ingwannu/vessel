//! Concrete owners receive lost operation admissions before a new follow starts.
use super::{
    FollowBinding, OperationKind, OperationTicket, ResidentProblem, ResidentToken, Residents,
};

/// An admitted operation whose wire outcome must now remain uncertain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetiredOperation {
    kind: OperationKind,
    ticket: OperationTicket,
}
impl RetiredOperation {
    /// Only the matching resource owner changes its pending UI state.
    #[must_use]
    pub const fn kind(self) -> OperationKind {
        self.kind
    }
    /// Match the exact original admission; this cannot be readmitted.
    #[must_use]
    pub const fn ticket(self) -> OperationTicket {
        self.ticket
    }
}
/// Notification data carries no replacement authority or new admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowRetirement {
    binding: FollowBinding,
    operations: Vec<RetiredOperation>,
}
impl FollowRetirement {
    /// The original attempt is invalid before an owner sees this notice.
    #[must_use]
    pub const fn binding(&self) -> FollowBinding {
        self.binding
    }
    /// Bounded by the operation cap; no input or peer message is copied.
    #[must_use]
    pub fn operations(&self) -> &[RetiredOperation] {
        &self.operations
    }
}
impl Residents {
    /// Retire before notification, cancellation and replacement attach.
    /// This first phase also notifies owners if the next generation cannot be
    /// allocated, so exhaustion cannot leave an obsolete Sending indicator.
    ///
    /// # Errors
    /// Refuses another window, absent resource or retired incarnation.
    pub fn retire_current(
        &mut self,
        token: ResidentToken,
    ) -> Result<Option<FollowRetirement>, ResidentProblem> {
        let entry = self.entry_mut(token)?;
        if !entry.attached {
            return Ok(None);
        }
        let binding = FollowBinding {
            resident: token,
            follow: entry.follow,
        };
        let operations = entry
            .operations
            .iter()
            .map(|(kind, ordinal)| RetiredOperation {
                kind: *kind,
                ticket: OperationTicket {
                    binding,
                    kind: *kind,
                    ordinal: *ordinal,
                },
            })
            .collect();
        entry.attached = false;
        super::transport::retire_operations(entry);
        Ok(Some(FollowRetirement {
            binding,
            operations,
        }))
    }
}
