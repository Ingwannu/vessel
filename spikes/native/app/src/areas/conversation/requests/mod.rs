//! Staged Requests owner kernel. W12 alone installs it in `SessionResources`.
//! No factory, ticket allocator, transport, global lookup or form copy lives here.
mod eligibility;
mod lifecycle;

use super::ConversationArea;
use vessel_app_kit::FieldId;
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::requests::Request;
use vessel_app_view::residents::{FollowBinding, OperationTicket, ResidentProblem};
use vessel_proto::{PermissionId, PermissionOptionId, QuestionId};

/// Owner correlation cap; independent of the sole registry's operation bound.
const MAX_REQUEST_EFFECTS: usize = 32;

/// A callback retains the actual form source or actual offered permission option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Form {
    Permission {
        id: PermissionId,
        option: PermissionOptionId,
    },
    Question {
        id: QuestionId,
        source: FieldId,
    },
}
impl Form {
    pub(super) const fn request(&self) -> Request {
        match self {
            Self::Permission { id, .. } => Request::Permission(*id),
            Self::Question { id, .. } => Request::Question(*id),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    Sending,
    Uncertain,
}
#[derive(Debug)]
struct Effect {
    request: Request,
    binding: FollowBinding,
    ticket: OperationTicket,
    stage: Stage,
}
/// One resource's bounded correlations; never a second admission registry.
#[derive(Debug, Default)]
pub(crate) struct Requests(Vec<Effect>);

/// Borrow only the owner and its real area across W12's synchronous mint.
/// Dropping this value after refusal changes neither draft nor Sending state.
#[must_use]
pub(crate) struct Prepared<'a> {
    owner: &'a mut Requests,
    area: &'a mut ConversationArea,
    binding: FollowBinding,
    form: Form,
}
impl std::fmt::Debug for Prepared<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Prepared")
            .field("binding", &self.binding)
            .finish_non_exhaustive()
    }
}
impl Requests {
    /// The caller first validates CURRENT Visible/Focused full `SessionContext`.
    /// This checks captured origin BEFORE W12 allocates any Request ticket.
    /// Context is identity, not a delivery capability; ingress still needs the
    /// original same-lease guard and exact follow admitted by the sole registry.
    pub(crate) fn prepare<'a>(
        &'a mut self,
        original: &ConnectionContext,
        binding: FollowBinding,
        form: Form,
        area: &'a mut ConversationArea,
    ) -> Result<Prepared<'a>, ResidentProblem> {
        if original.node() != binding.resident.address.node {
            return Err(ResidentProblem::OwnerConflict);
        }
        if self.0.len() >= MAX_REQUEST_EFFECTS {
            return Err(ResidentProblem::Capacity);
        }
        if self.0.iter().any(|effect| effect.request == form.request()) {
            return Err(ResidentProblem::Held);
        }
        eligibility::check(area, &form)?;
        Ok(Prepared {
            owner: self,
            area,
            binding,
            form,
        })
    }

    /// Check correlation BEFORE the sole registry consumes the original ticket.
    pub(crate) fn settling(
        &self,
        binding: FollowBinding,
        ticket: OperationTicket,
    ) -> Result<Request, ResidentProblem> {
        self.0
            .iter()
            .find(|effect| {
                effect.binding == binding
                    && effect.ticket == ticket
                    && effect.stage == Stage::Sending
            })
            .map(|effect| effect.request)
            .ok_or(ResidentProblem::UnadmittedOperation)
    }

    /// Call only INSIDE the sole registry's successful exact settle callback,
    /// after its ticket was consumed, before applying the original request result.
    pub(crate) fn settled(
        &mut self,
        binding: FollowBinding,
        ticket: OperationTicket,
    ) -> Result<Request, ResidentProblem> {
        let request = self.settling(binding, ticket)?;
        self.0
            .retain(|effect| effect.binding != binding || effect.ticket != ticket);
        Ok(request)
    }
    #[must_use]
    pub(crate) fn stage(&self, request: Request) -> Option<Stage> {
        self.0
            .iter()
            .find(|effect| effect.request == request)
            .map(|effect| effect.stage)
    }
}
impl Prepared<'_> {
    /// W12 supplies ONLY the just-minted Request ticket for this original binding.
    /// No await or area mutation may occur between prepare, mint and commit.
    /// This kernel cannot mint a ticket or confer authority on the supplied value.
    pub(crate) fn commit(self, ticket: OperationTicket) {
        match &self.form {
            Form::Permission { id, option } => {
                self.area.permit_sending = Some((*id, option.clone()));
            }
            Form::Question { .. } => {
                if let Some(answering) = self.area.answering.as_mut() {
                    answering.request_sent();
                }
            }
        }
        self.owner.0.push(Effect {
            request: self.form.request(),
            binding: self.binding,
            ticket,
            stage: Stage::Sending,
        });
    }
}
