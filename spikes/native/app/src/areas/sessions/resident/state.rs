//! Concrete lifecycle holds; canonical Residents alone mints/settles tickets.
use super::{Problem, Snapshot};
use vessel_app_view::residents::{
    FollowBinding, FollowRetirement, HoldReason, OperationKind, OperationTicket, OwnerReadiness,
};
use vessel_app_view::sessions::{LifecycleAction, LifecycleTicket};

#[derive(Clone, Copy)]
struct Decision {
    binding: FollowBinding,
    decision: LifecycleTicket,
    action: LifecycleAction,
}
/// Preserve the original uncertain effect for explicit owner reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Pending {
    pub(crate) binding: FollowBinding,
    pub(crate) ticket: OperationTicket,
    pub(crate) action: LifecycleAction,
}
enum Phase {
    Loading,
    Ready,
    Decision(Decision),
    Sending(Pending),
    RevokedDecision(Decision),
    Uncertain(Pending),
}
/// This value will move into the sole W12 resource bundle, never a side map.
pub(crate) struct LifecycleResources {
    snapshot: Option<Snapshot>,
    phase: Phase,
    next_decision: Option<LifecycleTicket>,
}
impl LifecycleResources {
    /// No default Ready: an owner must supply exact scoped hydration first.
    pub(crate) const fn staged() -> Self {
        Self {
            snapshot: None,
            phase: Phase::Loading,
            next_decision: Some(LifecycleTicket::FIRST),
        }
    }
    pub(crate) fn hydrate(&mut self, snapshot: Snapshot) -> Result<(), Problem> {
        match self.phase {
            Phase::Loading => {
                self.snapshot = Some(snapshot);
                self.phase = Phase::Ready;
                Ok(())
            }
            Phase::Ready
            | Phase::Decision(_)
            | Phase::Sending(_)
            | Phase::RevokedDecision(_)
            | Phase::Uncertain(_) => Err(Problem::Held),
        }
    }
    pub(crate) const fn readiness(&self) -> OwnerReadiness {
        match self.phase {
            Phase::Loading => OwnerReadiness::Hold(HoldReason::Loading),
            Phase::Ready => OwnerReadiness::Ready,
            Phase::Decision(_) | Phase::RevokedDecision(_) => {
                OwnerReadiness::Hold(HoldReason::Overlay)
            }
            Phase::Sending(_) => OwnerReadiness::Hold(HoldReason::PendingEffect),
            Phase::Uncertain(_) => OwnerReadiness::Hold(HoldReason::UncertainEffect),
        }
    }
    pub(crate) fn prepare(
        &mut self,
        binding: FollowBinding,
        action: LifecycleAction,
    ) -> Result<LifecycleTicket, Problem> {
        if !matches!(self.phase, Phase::Ready) {
            return Err(Problem::Held);
        }
        let snapshot = self.snapshot.as_ref().ok_or(Problem::Loading)?;
        if snapshot.resident() != binding.resident {
            return Err(Problem::WrongResource);
        }
        let decision = self.next_decision.ok_or(Problem::Exhausted)?;
        self.next_decision = decision.next();
        self.phase = Phase::Decision(Decision {
            binding,
            decision,
            action,
        });
        Ok(decision)
    }
    /// Obtain this ticket from Residents with OperationKind::Lifecycle, after
    /// checking the exact decision in the same synchronous confirmation callback.
    /// This records pending UI; it grants no effect or completion authority.
    pub(crate) fn sent(
        &mut self,
        decision: LifecycleTicket,
        ticket: OperationTicket,
    ) -> Result<(), Problem> {
        let Phase::Decision(expected) = self.phase else {
            return Err(Problem::Held);
        };
        if expected.decision != decision {
            return Err(Problem::Held);
        }
        self.phase = Phase::Sending(Pending {
            binding: expected.binding,
            ticket,
            action: expected.action,
        });
        Ok(())
    }
    /// Invoke only inside canonical settle after the original delivery lease check.
    /// A retired/duplicate completion cannot clear input or authorize a close.
    pub(crate) fn acknowledged(
        &mut self,
        binding: FollowBinding,
        ticket: OperationTicket,
    ) -> Option<LifecycleAction> {
        if let Phase::Sending(pending) = self.phase
            && pending.binding == binding
            && pending.ticket == ticket
        {
            self.phase = Phase::Ready;
            Some(pending.action)
        } else {
            None
        }
    }
    /// Control-only settlement after canonical exact-ticket consumption.
    /// Only proven local no-send can release pending UI. A peer error can follow
    /// a committed mutation, so it never grants success or erases uncertainty.
    pub(crate) fn failed(
        &mut self,
        binding: FollowBinding,
        ticket: OperationTicket,
        failure: &vessel_app_link::sessions::lifecycle::Failure,
    ) -> bool {
        let Phase::Sending(pending) = self.phase else {
            return false;
        };
        if pending.binding != binding || pending.ticket != ticket {
            return false;
        }
        use vessel_app_link::sessions::lifecycle::Failure;
        self.phase = match failure {
            Failure::Unsent | Failure::IdentityChanged => Phase::Ready,
            Failure::Refused(_) | Failure::LostReply(_) | Failure::Uncertain => {
                Phase::Uncertain(pending)
            }
        };
        true
    }
    pub(crate) fn retired(&mut self, retirement: &FollowRetirement) {
        match self.phase {
            Phase::Decision(decision) if decision.binding == retirement.binding() => {
                self.phase = Phase::RevokedDecision(decision);
            }
            Phase::Sending(pending)
                if pending.binding == retirement.binding()
                    && retirement.operations().iter().any(|operation| {
                        operation.kind() == OperationKind::Lifecycle
                            && operation.ticket() == pending.ticket
                    }) =>
            {
                self.phase = Phase::Uncertain(pending);
            }
            Phase::Loading
            | Phase::Ready
            | Phase::Decision(_)
            | Phase::Sending(_)
            | Phase::RevokedDecision(_)
            | Phase::Uncertain(_) => {}
        }
    }
    pub(crate) fn uncertain(&self) -> Option<Pending> {
        match self.phase {
            Phase::Uncertain(pending) => Some(pending),
            Phase::Loading
            | Phase::Ready
            | Phase::Decision(_)
            | Phase::Sending(_)
            | Phase::RevokedDecision(_) => None,
        }
    }
    pub(crate) fn dismiss(&mut self, decision: LifecycleTicket) {
        if matches!(self.phase, Phase::Decision(expected) | Phase::RevokedDecision(expected) if expected.decision == decision)
        {
            self.phase = Phase::Ready;
        }
    }
    /// Uncertain effects and retained confirmations cannot become clean by hydration.
    pub(crate) fn take_clean_snapshot(&mut self) -> Result<Snapshot, Problem> {
        if !matches!(self.phase, Phase::Ready) {
            return Err(Problem::Held);
        }
        let snapshot = self.snapshot.take().ok_or(Problem::Loading)?;
        self.phase = Phase::Loading;
        Ok(snapshot)
    }
}
