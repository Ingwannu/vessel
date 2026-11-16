//! One prearmed stop lives in the existing binding, through ACK-held Window work.
//! This pure policy cannot prove a transport lease, typed stream end or UI intent.
//! Callers must validate those original facts immediately before each transition.
use super::{
    FollowBinding, FollowRetirement, HoldReason, OperationKind, OperationTicket, OwnerReadiness,
    ResidentProblem, Residents, ResourceOwner, RetiredOperation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FollowState {
    Retired,
    Receiving,
    Sealed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    WirePending,
    WindowHeld,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct Expectation {
    pub ticket: OperationTicket,
    action: StopAction,
    phase: Phase,
}
/// Only these original typed intents may prearm expected normal end, before wire.
/// This subset neither sends a stop nor infers a node execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopAction {
    /// Archive under the original lifecycle intent.
    Archive,
    /// Stop and later close through the original guarded Window plan.
    Exit,
    /// Stop and later create the original scoped fresh draft.
    Fresh,
}
/// The actor classifies the original typed reply; stream end never supplies ACK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopCompletion {
    /// Authoritative typed success for the original action and session.
    Acknowledged,
    /// Terminal refusal/deadline; the owner retains its `NoSend`/`Uncertain` evidence.
    Refused,
}
/// Opaque local correlation after the wire ticket was consumed exactly once.
/// It grants no Window/target/lease permission and cannot be re-admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopWindowReceipt(OperationTicket);
/// A terminal wire settlement leaves local work held, or revokes the expectation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopSettlement {
    /// Owner must retain the same original guard/context and exact Window work.
    WindowHeld(StopWindowReceipt),
    /// A sealed binding fully retires; receiving bindings keep ordinary ingress.
    Refused(Option<FollowRetirement>),
}
/// Partial retirement is deliberately not a full `FollowRetirement` notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowSeal {
    binding: FollowBinding,
    operations: Vec<RetiredOperation>,
}
impl FollowSeal {
    /// Only this original binding's ingress/admission became sealed.
    #[must_use]
    pub const fn binding(&self) -> FollowBinding {
        self.binding
    }
    /// Other operations retire; the prearmed stop or acknowledged Window work survives.
    #[must_use]
    pub fn operations(&self) -> &[RetiredOperation] {
        &self.operations
    }
}
/// The original ordered normal-end consumer dispatches one distinct owner notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowEnd {
    /// No prearmed expectation: ordinary full retirement.
    Retired(FollowRetirement),
    /// One original stop/Window expectation remains, without receive permission.
    Sealed(FollowSeal),
}
impl Residents {
    /// Mint and prearm one original typed stop atomically before its wire call.
    /// The caller already validated actual intent, context, form and close plan.
    ///
    /// # Errors
    /// Refuses stale/sealed bindings, a second expectation or ordinary ticket limits.
    pub fn admit_stop(
        &mut self,
        binding: FollowBinding,
        action: StopAction,
    ) -> Result<OperationTicket, ResidentProblem> {
        self.validate_binding(binding)?;
        if self.entry(binding.resident)?.stop.is_some() {
            return Err(ResidentProblem::Held);
        }
        let ticket = self.admit_operation(binding, OperationKind::Lifecycle)?;
        let entry = self.entry_mut(binding.resident)?;
        entry.stop = Some(Expectation {
            ticket,
            action,
            phase: Phase::WirePending,
        });
        if matches!(
            entry.owners.get(&ResourceOwner::Lifecycle),
            None | Some(OwnerReadiness::Ready | OwnerReadiness::Hold(HoldReason::Loading))
        ) {
            // Admission adds a hold; it never weakens an owner's protected vote.
            let _prior = entry.owners.insert(
                ResourceOwner::Lifecycle,
                OwnerReadiness::Hold(HoldReason::PendingEffect),
            );
        }
        Ok(ticket)
    }
    /// Seal only after the same guarded pump installed all preceding records and
    /// observed an actual typed normal End. Generic EOF/error/status is insufficient.
    ///
    /// # Errors
    /// Duplicate/old end cannot change a sealed or successor binding.
    pub fn end_follow(&mut self, binding: FollowBinding) -> Result<FollowEnd, ResidentProblem> {
        self.validate_binding(binding)?;
        let entry = self.entry_mut(binding.resident)?;
        let Some(stop) = entry.stop else {
            return self.retire_binding(binding).map(FollowEnd::Retired);
        };
        entry.follow_state = FollowState::Sealed;
        let mut operations = Vec::new();
        entry.operations.retain(|(kind, ordinal)| {
            if (*kind, *ordinal) == (stop.ticket.kind, stop.ticket.ordinal) {
                return true;
            }
            let _prior = entry.owners.insert(
                kind.owner(),
                OwnerReadiness::Hold(HoldReason::UncertainEffect),
            );
            operations.push(RetiredOperation {
                kind: *kind,
                ticket: OperationTicket {
                    binding,
                    kind: *kind,
                    ordinal: *ordinal,
                },
            });
            false
        });
        Ok(FollowEnd::Sealed(FollowSeal {
            binding,
            operations,
        }))
    }
    /// Consume the original stop ticket before its owner records ACK-held work.
    /// The same original guard must be checked immediately before this call.
    ///
    /// # Errors
    /// Refuses wrong/duplicate/retired replies; ACK cannot mint a new expectation.
    pub fn settle_stop(
        &mut self,
        binding: FollowBinding,
        ticket: OperationTicket,
        completion: StopCompletion,
    ) -> Result<StopSettlement, ResidentProblem> {
        self.validate_current_binding(binding)?;
        let entry = self.entry_mut(binding.resident)?;
        let stop = entry.stop.ok_or(ResidentProblem::UnadmittedOperation)?;
        if stop.ticket != ticket
            || stop.phase != Phase::WirePending
            || !entry
                .operations
                .remove(&(OperationKind::Lifecycle, ticket.ordinal))
        {
            return Err(ResidentProblem::UnadmittedOperation);
        }
        match completion {
            StopCompletion::Acknowledged => {
                entry.stop = Some(Expectation {
                    phase: Phase::WindowHeld,
                    ..stop
                });
                Ok(StopSettlement::WindowHeld(StopWindowReceipt(ticket)))
            }
            StopCompletion::Refused => {
                entry.stop = None;
                let sealed = entry.follow_state == FollowState::Sealed;
                let notice = if sealed {
                    self.retire_current(binding.resident)?
                } else {
                    None
                };
                Ok(StopSettlement::Refused(notice))
            }
        }
    }
    /// Revalidate immediately before the originating Window's synchronous commit,
    /// in addition to its original guard, key/claim/target and concrete owner votes.
    ///
    /// # Errors
    /// A full retirement, consumed receipt or successor never inherits local work.
    pub fn validate_stop_window(
        &self,
        binding: FollowBinding,
        receipt: StopWindowReceipt,
    ) -> Result<StopAction, ResidentProblem> {
        self.validate_current_binding(binding)?;
        let stop = self
            .entry(binding.resident)?
            .stop
            .ok_or(ResidentProblem::UnadmittedOperation)?;
        if stop.ticket != receipt.0 || stop.phase != Phase::WindowHeld {
            return Err(ResidentProblem::UnadmittedOperation);
        }
        Ok(stop.action)
    }
    /// Consume local correlation once, before the already-validated Window commit.
    /// No await or owner mutation may separate validation, consumption and commit.
    /// Failure still retains protected resources and cannot reuse this receipt.
    ///
    /// # Errors
    /// Refuses a stale/duplicate local receipt without touching another binding.
    pub fn finish_stop_window(
        &mut self,
        binding: FollowBinding,
        receipt: StopWindowReceipt,
    ) -> Result<StopAction, ResidentProblem> {
        let action = self.validate_stop_window(binding, receipt)?;
        self.entry_mut(binding.resident)?.stop = None;
        Ok(action)
    }
    /// Original deadline/refusal revokes local Window authority while retaining
    /// protected owner values. A sealed binding is fully retired, never reopened.
    ///
    /// # Errors
    /// Refuses stale/duplicate receipts and cannot revoke a successor expectation.
    pub fn revoke_stop_window(
        &mut self,
        binding: FollowBinding,
        receipt: StopWindowReceipt,
    ) -> Result<Option<FollowRetirement>, ResidentProblem> {
        let _action = self.finish_stop_window(binding, receipt)?;
        if self.entry(binding.resident)?.follow_state == FollowState::Sealed {
            self.retire_current(binding.resident)
        } else {
            Ok(None)
        }
    }
    pub(super) fn validate_current_binding(
        &self,
        binding: FollowBinding,
    ) -> Result<(), ResidentProblem> {
        let entry = self.entry(binding.resident)?;
        if entry.follow_state == FollowState::Retired || entry.follow != binding.follow {
            return Err(ResidentProblem::StaleBinding);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
