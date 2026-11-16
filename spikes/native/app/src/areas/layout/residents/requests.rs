//! Atomic actual-owner preparation and the sole canonical Request ticket.
//! The real ingress must still check the original delivery guard at its edge.
use super::{SessionResidents, SessionResources};
use crate::areas::conversation::requests::Form;
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::requests::Request;
use vessel_app_view::residents::{
    FollowBinding, OperationKind, OperationTicket, PanelFocus, ResidentProblem, SessionContext,
    Visibility,
};

impl SessionResidents {
    /// Reserve the actual owner and form through synchronous prepare/mint/commit.
    /// The full captured local context must still be current and Visible/Focused;
    /// the original connection must be the one retained by the sole open minter.
    /// Neither this identity check nor its ticket substitutes for a live lease.
    pub(crate) fn admit_request(
        &mut self,
        original: &ConnectionContext,
        context: &SessionContext,
        binding: FollowBinding,
        form: Form,
    ) -> Result<OperationTicket, ResidentProblem> {
        self.admission.validate_context(context)?;
        self.admission.validate_binding(binding)?;
        if context.resident != binding.resident {
            return Err(ResidentProblem::OwnerConflict);
        }
        if context.visibility != Visibility::Visible || context.focus != PanelFocus::Focused {
            return Err(ResidentProblem::Held);
        }
        if self.entry(binding.resident)?.connection.as_ref() != Some(original) {
            return Err(ResidentProblem::OwnerConflict);
        }
        let resources = &mut self
            .entries
            .get_mut(&binding.resident.address.session)
            .ok_or(ResidentProblem::Missing)?
            .resources;
        let prepared =
            resources
                .requests
                .prepare(original, binding, form, &mut resources.conversation)?;
        let ticket = self
            .admission
            .admit_operation(binding, OperationKind::Request)?;
        prepared.commit(ticket);
        Ok(ticket)
    }

    /// For a definitive typed completion after original-guard validation,
    /// consume the canonical ticket before
    /// changing owner state or invoking its form/result callback. Replies never
    /// refresh a local context or inherit a successor's connection/presentation.
    /// An uncertain wire error is not completion and must retain its owner hold.
    pub(crate) fn settle_request<T>(
        &mut self,
        binding: FollowBinding,
        ticket: OperationTicket,
        apply: impl FnOnce(&mut SessionResources, Request) -> T,
    ) -> Result<T, ResidentProblem> {
        self.admission.validate_binding(binding)?;
        let _expected_request = self
            .entry(binding.resident)?
            .resources
            .requests
            .settling(binding, ticket)?;
        self.settle(binding, OperationKind::Request, ticket, |resources| {
            let request = resources.requests.settled(binding, ticket)?;
            Ok(apply(resources, request))
        })?
    }
}
