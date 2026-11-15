//! Staged Requests owner adapter to W12's sole concrete resource registry.
//! This records original admissions only; it cannot mint a ticket, attach a
//! follow, retarget a reply, clone a resource bundle or send any node work.
use super::ConversationArea;
use crate::areas::layout::residents::SessionResidents;
use vessel_app_view::residents::{
    FollowBinding, FollowRetirement, HoldReason, OperationKind, OperationTicket, OwnerReadiness,
    ResidentProblem,
};
use vessel_app_view::responses::{MAX_PENDING, Problem, Request};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Sending,
    Uncertain,
}
#[derive(Debug, Clone, Copy)]
struct Effect {
    request: Request,
    binding: FollowBinding,
    ticket: OperationTicket,
    stage: Stage,
}
/// Bounded correlation held inside the owning ConversationArea, not an
/// admission table. Only SessionResidents below can supply canonical tickets.
#[derive(Debug, Default)]
pub(super) struct RequestEffects(Vec<Effect>);
impl RequestEffects {
    fn sending(&self, binding: FollowBinding, ticket: OperationTicket) -> Option<usize> {
        self.0.iter().position(|effect| {
            effect.binding == binding && effect.ticket == ticket && effect.stage == Stage::Sending
        })
    }
}

impl ConversationArea {
    fn request_admission(&self, request: Request) -> Result<(), ResidentProblem> {
        if !self.response_waiting(request) {
            return Err(ResidentProblem::Missing);
        }
        if self.request_effects.0.len() >= MAX_PENDING {
            return Err(ResidentProblem::Capacity);
        }
        if self
            .request_effects
            .0
            .iter()
            .any(|effect| effect.request == request)
        {
            return Err(ResidentProblem::Held);
        }
        let sending = match request {
            Request::Permission(id) => self.permit_sending.contains_key(&id),
            Request::Question(id) => self.answering.is_sending(id),
        };
        if sending {
            Ok(())
        } else {
            Err(ResidentProblem::UnadmittedOperation)
        }
    }

    /// Empty and untouched forms still hold: they are live pending requests,
    /// not clean serializable editor drafts. No pending form is dehydrated.
    pub(crate) fn requests_readiness(&self) -> OwnerReadiness {
        if self
            .request_effects
            .0
            .iter()
            .any(|effect| effect.stage == Stage::Uncertain)
        {
            OwnerReadiness::Hold(HoldReason::UncertainEffect)
        } else if !self.request_effects.0.is_empty() {
            OwnerReadiness::Hold(HoldReason::PendingEffect)
        } else if self.timeline.pending_permissions().next().is_some()
            || !self.timeline.waiting_questions().is_empty()
            || !self.live.permissions.is_empty()
            || !self.live.questions.is_empty()
        {
            OwnerReadiness::Hold(HoldReason::DirtyInput)
        } else {
            OwnerReadiness::Ready
        }
    }

    /// The sole registry has already invalidated this attempt. Match BOTH its
    /// original binding and opaque request ticket before changing UI state.
    pub(crate) fn requests_retired(&mut self, notice: &FollowRetirement) {
        for effect in &mut self.request_effects.0 {
            if effect.stage == Stage::Sending
                && effect.binding == notice.binding()
                && notice.operations().iter().any(|retired| {
                    retired.kind() == OperationKind::Request && retired.ticket() == effect.ticket
                })
            {
                effect.stage = Stage::Uncertain;
                // Cards hide Sending in the presence of this exact-request
                // failure. Original fields/actions stay retained; admission
                // below refuses another send until authoritative settlement.
                self.response_failures
                    .failed(effect.request, Problem::Unconfirmed);
            }
        }
    }
}

impl SessionResidents {
    /// Only this synchronous path may correlate a new Requests operation. All
    /// refusal checks run before the sole registry mints an operation ticket.
    pub(crate) fn request_admit(
        &mut self,
        binding: FollowBinding,
        request: Request,
    ) -> Result<OperationTicket, ResidentProblem> {
        self.receive(binding, |resources| {
            if resources
                .conversation
                .shown
                .as_ref()
                .is_none_or(|shown| shown.session != binding.resident.address.session)
            {
                return Err(ResidentProblem::OwnerConflict);
            }
            resources.conversation.request_admission(request)
        })??;
        let ticket = self.admit_operation(binding, OperationKind::Request)?;
        self.receive(binding, |resources| {
            resources.conversation.request_effects.0.push(Effect {
                request,
                binding,
                ticket,
                stage: Stage::Sending,
            });
        })?;
        Ok(ticket)
    }

    /// Consume the sole owner's original ticket before installing a result.
    /// The callback receives its original exact request, never a selected id.
    pub(crate) fn request_settle<T>(
        &mut self,
        binding: FollowBinding,
        ticket: OperationTicket,
        apply: impl FnOnce(&mut ConversationArea, Request) -> T,
    ) -> Result<T, ResidentProblem> {
        let owned = self.receive(binding, |resources| {
            resources
                .conversation
                .request_effects
                .sending(binding, ticket)
                .is_some()
        })?;
        if !owned {
            return Err(ResidentProblem::UnadmittedOperation);
        }
        self.settle(binding, OperationKind::Request, ticket, |resources| {
            let area = &mut resources.conversation;
            let index = area
                .request_effects
                .sending(binding, ticket)
                .ok_or(ResidentProblem::UnadmittedOperation)?;
            let effect = area.request_effects.0.swap_remove(index);
            Ok(apply(area, effect.request))
        })?
    }
}
