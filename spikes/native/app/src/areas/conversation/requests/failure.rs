//! Local owner dispositions after sole-registry ticket consumption.
//! These values classify an already correlated owner transition; they are NOT
//! transport proofs, delivery guards, receipts or permission authority.
use super::{ConversationArea, Form, Requests, Stage, eligibility};
use vessel_app_view::residents::{FollowBinding, OperationTicket, ResidentProblem};

/// The real transport owner supplies this only after checking its actual phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Disposition {
    /// Proven never dispatched. Changed/Closed alone cannot establish this.
    NoSend,
    /// Dispatch started or its effect is ambiguous; never release or resend.
    Uncertain,
}

/// No variant means success. Release is local only; canonical Pending/current
/// visible input admission and a new operation are still required before retry.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Transition {
    Released(Form),
    Unchanged(Form),
    Uncertain(Form),
}

impl Requests {
    /// Call only INSIDE the sole registry's successful exact settle callback,
    /// after its Request ticket was consumed. The caller prechecks settling().
    /// A guardless NoSend must have actual correlated never-dispatched proof;
    /// this method creates no guard, success, receipt or authoritative data.
    /// Rich guarded NoEffect refusals remain a separate future wire adapter.
    pub(crate) fn failed(
        &mut self,
        binding: FollowBinding,
        ticket: OperationTicket,
        disposition: Disposition,
        area: &mut ConversationArea,
    ) -> Result<Transition, ResidentProblem> {
        let index = self
            .0
            .iter()
            .position(|effect| {
                effect.binding == binding
                    && effect.ticket == ticket
                    && effect.stage == Stage::Sending
            })
            .ok_or(ResidentProblem::UnadmittedOperation)?;
        match disposition {
            Disposition::Uncertain => {
                let effect = &mut self.0[index];
                effect.stage = Stage::Uncertain;
                Ok(Transition::Uncertain(effect.form.clone()))
            }
            Disposition::NoSend => {
                let effect = self.0.remove(index);
                if release(area, &effect.form) {
                    Ok(Transition::Released(effect.form))
                } else {
                    Ok(Transition::Unchanged(effect.form))
                }
            }
        }
    }
}

fn release(area: &mut ConversationArea, form: &Form) -> bool {
    match form {
        Form::Permission { id, option } => {
            let offered = eligibility::permission(area, *id)
                .is_some_and(|request| request.options.iter().any(|value| value.id == *option));
            if offered
                && area
                    .permit_sending
                    .as_ref()
                    .is_some_and(|(sent, value)| sent == id && value == option)
            {
                area.permit_sending = None;
                true
            } else {
                false
            }
        }
        Form::Question { id, source } => {
            eligibility::question_waiting(area, *id)
                && area
                    .answering
                    .as_mut()
                    .is_some_and(|answering| answering.release_request(*id, *source))
        }
    }
}
