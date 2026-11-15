//! Reply admission is independent of input lifetime and connection identity.
//! Link must additionally validate the shared connection owner's actual lease.
use super::{
    FollowBinding, FollowGeneration, HoldReason, MAX_RESIDENT_OPERATIONS, OperationKind,
    OperationTicket, OwnerReadiness, ResidentProblem, ResidentToken, Residents,
};
impl Residents {
    /// Retire the prior attempt before launching any attach, including same-node
    /// reconnect. Outstanding wire effects become uncertain at their owning adapter.
    ///
    /// # Errors
    /// Refuses stale lifetimes or generation exhaustion without reusing a binding.
    pub fn begin_follow(&mut self, token: ResidentToken) -> Result<FollowBinding, ResidentProblem> {
        let entry = self.entry_mut(token)?;
        // Invalidate even when exhausted: the old attempt cannot remain authorized.
        entry.attached = false;
        retire_operations(entry);
        let next = entry
            .follow
            .0
            .checked_add(1)
            .ok_or(ResidentProblem::Exhausted)?;
        entry.follow = FollowGeneration(next);
        entry.attached = true;
        Ok(FollowBinding {
            resident: token,
            follow: entry.follow,
        })
    }
    /// Retirement is synchronous at dispatch before a replacement attach starts.
    ///
    /// # Errors
    /// A queued old disconnect cannot retire a newer attempt.
    pub fn retire_follow(&mut self, binding: FollowBinding) -> Result<(), ResidentProblem> {
        self.validate_binding(binding)?;
        let entry = self.entry_mut(binding.resident)?;
        entry.attached = false;
        retire_operations(entry);
        Ok(())
    }
    /// Every metadata, record, error and action reply checks this before mutation.
    ///
    /// # Errors
    /// Refuses retired/replaced attempts, even if the `NodeId` is the same again.
    pub fn validate_binding(&self, binding: FollowBinding) -> Result<(), ResidentProblem> {
        let entry = self.entry(binding.resident)?;
        if !entry.attached || entry.follow != binding.follow {
            return Err(ResidentProblem::StaleBinding);
        }
        Ok(())
    }
    /// An owner requests a fresh ticket before sending an effect. Refusal sends nothing.
    ///
    /// # Errors
    /// Refuses stale bindings, generation exhaustion or the bounded pending limit.
    pub fn admit_operation(
        &mut self,
        binding: FollowBinding,
        kind: OperationKind,
    ) -> Result<OperationTicket, ResidentProblem> {
        self.validate_binding(binding)?;
        let entry = self.entry_mut(binding.resident)?;
        if entry.operations.len() >= MAX_RESIDENT_OPERATIONS {
            return Err(ResidentProblem::Capacity);
        }
        let next = entry
            .next_operation
            .checked_add(1)
            .ok_or(ResidentProblem::Exhausted)?;
        let ticket = OperationTicket {
            binding,
            kind,
            ordinal: next,
        };
        let _inserted = entry.operations.insert((kind, next));
        entry.next_operation = next;
        Ok(ticket)
    }
    /// Consume an exact reply once before its owner touches input or close authority.
    ///
    /// # Errors
    /// Refuses old attempts, wrong owner kinds and duplicate/unknown replies.
    pub fn settle_operation(
        &mut self,
        binding: FollowBinding,
        kind: OperationKind,
        ticket: OperationTicket,
    ) -> Result<(), ResidentProblem> {
        self.validate_binding(binding)?;
        if ticket.binding != binding || ticket.kind != kind {
            return Err(ResidentProblem::UnadmittedOperation);
        }
        if !self
            .entry_mut(binding.resident)?
            .operations
            .remove(&(kind, ticket.ordinal))
        {
            return Err(ResidentProblem::UnadmittedOperation);
        }
        Ok(())
    }
}

pub(super) fn retire_operations(entry: &mut super::registry::Resident) {
    for (kind, _) in &entry.operations {
        let _previous = entry.owners.insert(
            kind.owner(),
            OwnerReadiness::Hold(HoldReason::UncertainEffect),
        );
    }
    entry.operations.clear();
}
