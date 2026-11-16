//! Follow/ticket ingress stays separate from view visibility and input lifetime.
use super::{SessionResidents, SessionResources};
use std::time::Duration;
use vessel_app_view::layout::PaneId;
use vessel_app_view::residents::{
    FollowBinding, FollowRetirement, OperationKind, OperationTicket, OwnerReadiness, PanelClaim,
    ResidentProblem, ResidentToken, ResourceOwner, SessionContext, Visibility,
};

impl SessionResidents {
    pub(crate) fn present(
        &mut self,
        token: ResidentToken,
        claim: &PanelClaim,
        pane: PaneId,
        visibility: Visibility,
    ) -> Result<(), ResidentProblem> {
        self.admission.present(token, claim, pane, visibility)
    }
    pub(crate) fn focus(
        &mut self,
        token: ResidentToken,
        claim: &PanelClaim,
    ) -> Result<(), ResidentProblem> {
        self.admission.focus(token, claim)
    }
    pub(crate) fn focused(&self) -> Option<SessionContext> {
        self.admission.focused()
    }
    pub(crate) fn readiness(
        &mut self,
        token: ResidentToken,
        owner: ResourceOwner,
        readiness: OwnerReadiness,
    ) -> Result<(), ResidentProblem> {
        self.admission.readiness(token, owner, readiness)
    }
    pub(crate) fn release_claim(
        &mut self,
        token: ResidentToken,
        claim: &PanelClaim,
        now: Duration,
    ) -> Result<(), ResidentProblem> {
        self.admission.release_claim(token, claim, now)
    }
    /// Retire the epoch before cancelling/dropping actual owner resources.
    /// The caller must first transfer every owner's typed clean snapshot.
    pub(crate) fn retire_clean(
        &mut self,
        token: ResidentToken,
    ) -> Result<SessionResources, ResidentProblem> {
        self.check_installed_readiness(token)?;
        self.admission.retire_clean(token)?;
        self.entries
            .remove(&token.address.session)
            .map(|entry| entry.resources)
            .ok_or(ResidentProblem::Missing)
    }
    pub(crate) fn begin_follow(
        &mut self,
        token: ResidentToken,
        retired: impl FnOnce(&mut SessionResources, &FollowRetirement),
    ) -> Result<FollowBinding, ResidentProblem> {
        let _entry = self.entry(token)?;
        if let Some(notice) = self.admission.retire_current(token)? {
            self.notify_retirement(token, &notice, retired)?;
        }
        self.admission.begin_follow(token)
    }
    pub(crate) fn retire_follow(
        &mut self,
        binding: FollowBinding,
        retired: impl FnOnce(&mut SessionResources, &FollowRetirement),
    ) -> Result<(), ResidentProblem> {
        self.admission.validate_binding(binding)?;
        let notice = self
            .admission
            .retire_current(binding.resident)?
            .ok_or(ResidentProblem::StaleBinding)?;
        self.notify_retirement(binding.resident, &notice, retired)
    }
    fn notify_retirement(
        &mut self,
        token: ResidentToken,
        notice: &FollowRetirement,
        retired: impl FnOnce(&mut SessionResources, &FollowRetirement),
    ) -> Result<(), ResidentProblem> {
        let resources = &mut self.entry_mut(token)?.resources;
        resources.follow_retired(notice);
        retired(resources, notice);
        Ok(())
    }
    /// Concrete holds cannot be bypassed by an old table-only Ready marker.
    /// This conservative gate never clears a canonical uncertainty vote; actual
    /// owner reconciliation must publish that separately in the scoped consumer.
    fn check_installed_readiness(&self, token: ResidentToken) -> Result<(), ResidentProblem> {
        if self
            .entry(token)?
            .resources
            .installed_readiness()
            .iter()
            .any(|(_, vote)| *vote != OwnerReadiness::Ready)
        {
            return Err(ResidentProblem::Held);
        }
        Ok(())
    }
    pub(crate) fn admit_operation(
        &mut self,
        binding: FollowBinding,
        kind: OperationKind,
    ) -> Result<OperationTicket, ResidentProblem> {
        self.admission.admit_operation(binding, kind)
    }
    /// Link's shared connection lease must already have checked before effects
    /// and after await. This checks our independent attempt before installation.
    /// Hidden residents still receive fresh records without gaining input focus.
    pub(crate) fn receive<T>(
        &mut self,
        binding: FollowBinding,
        apply: impl FnOnce(&mut SessionResources) -> T,
    ) -> Result<T, ResidentProblem> {
        self.admission.validate_binding(binding)?;
        Ok(apply(&mut self.entry_mut(binding.resident)?.resources))
    }
    /// Consume admission once BEFORE an owner may clear text or complete close.
    pub(crate) fn settle<T>(
        &mut self,
        binding: FollowBinding,
        kind: OperationKind,
        ticket: OperationTicket,
        apply: impl FnOnce(&mut SessionResources) -> T,
    ) -> Result<T, ResidentProblem> {
        let _entry = self.entry(binding.resident)?;
        self.admission.settle_operation(binding, kind, ticket)?;
        Ok(apply(&mut self.entry_mut(binding.resident)?.resources))
    }
}
