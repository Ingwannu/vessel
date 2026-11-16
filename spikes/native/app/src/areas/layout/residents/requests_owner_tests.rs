//! W12-authorized TEST ONLY split-borrow harness, not a product adapter.
//! Requests is fixture-local until W12 installs the sole resource bundle field.
use super::SessionResidents;
use crate::areas::conversation::requests::{Form, Requests};
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::residents::{FollowBinding, OperationKind, OperationTicket, ResidentProblem};

impl SessionResidents {
    pub(crate) fn requests_test_admit(
        &mut self,
        owner: &mut Requests,
        original: &ConnectionContext,
        binding: FollowBinding,
        form: Form,
    ) -> Result<OperationTicket, ResidentProblem> {
        self.admission.validate_binding(binding)?;
        let entry = self
            .entries
            .get_mut(&binding.resident.address.session)
            .ok_or(ResidentProblem::Missing)?;
        if entry.token != binding.resident {
            return Err(ResidentProblem::StaleEpoch);
        }
        let prepared = owner.prepare(original, binding, form, &mut entry.resources.conversation)?;
        let ticket = self
            .admission
            .admit_operation(binding, OperationKind::Request)?;
        prepared.commit(ticket);
        Ok(ticket)
    }
}
