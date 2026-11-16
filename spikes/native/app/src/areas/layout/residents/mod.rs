//! Staged sole resource owner; never run beside the legacy product globals.
//!
//! The factory is supplied by scoped owner adapters. In particular, the old
//! selected-session composer callback is not a production resource constructor.
mod access;
mod opening;
mod requests;
pub(crate) use opening::{SessionAdmission, SessionOpening, SessionOpeningProblem};
#[cfg(test)]
mod tests;

use crate::areas::{
    conversation::{ConversationArea, requests::Requests},
    session::SessionView,
    session_lifecycle::LifecycleResources,
};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry as MapEntry;
use vessel_app_view::layout::PaneId;
use vessel_app_view::residents::{
    FollowRetirement, OwnerReadiness, PanelClaim, ResidentProblem, ResidentToken, Residents,
    ResourceOwner, SessionAddress, SessionContext,
};
use vessel_proto::SessionId;

/// Concrete resources move once into their sole owner. No Clone/serde escape.
pub(crate) struct SessionResources {
    pub(crate) session: SessionView,
    pub(crate) conversation: ConversationArea,
    pub(crate) requests: Requests,
    pub(crate) lifecycle: LifecycleResources,
}
impl SessionResources {
    /// Move actual fields once; owner kernels add no renderer, registry or input.
    pub(crate) fn new(session: SessionView, conversation: ConversationArea) -> Self {
        Self {
            session,
            conversation,
            requests: Requests::default(),
            lifecycle: LifecycleResources::staged(),
        }
    }
    fn installed_readiness(&self) -> [(ResourceOwner, OwnerReadiness); 2] {
        [
            (
                ResourceOwner::Requests,
                self.requests.readiness(&self.conversation),
            ),
            (ResourceOwner::Lifecycle, self.lifecycle.readiness()),
        ]
    }
    fn follow_retired(&mut self, notice: &FollowRetirement) {
        self.requests.retired(notice);
        self.lifecycle.retired(notice);
        self.conversation.navigation_retired();
    }
}
struct Entry {
    token: ResidentToken,
    resources: SessionResources,
    connection: Option<vessel_app_view::connection::ConnectionContext>,
}
/// Metadata admission and resource storage are private parts of the same owner.
/// All public accesses validate admission before borrowing a concrete resource.
pub(crate) struct SessionResidents {
    admission: Residents,
    entries: BTreeMap<SessionId, Entry>,
}
impl SessionResidents {
    pub(crate) fn new() -> Self {
        Self {
            admission: Residents::new(),
            entries: BTreeMap::new(),
        }
    }
    /// Capacity/conflict refusal happens before invoking the resource factory.
    /// Reclaiming a hidden tab reuses its actual fields and retained timeline.
    pub(crate) fn claim_session(
        &mut self,
        address: SessionAddress,
        claim: &PanelClaim,
        pane: PaneId,
        build: impl FnOnce(&SessionContext) -> SessionResources,
    ) -> Result<ResidentToken, ResidentProblem> {
        let token = self.admission.claim(address, claim.clone(), pane)?;
        if let MapEntry::Vacant(slot) = self.entries.entry(address.session) {
            let context = self.admission.context(token, claim)?;
            let resources = build(&context);
            let _inserted = slot.insert(Entry {
                token,
                resources,
                connection: None,
            });
        }
        Ok(token)
    }
    pub(crate) fn context(
        &self,
        token: ResidentToken,
        claim: &PanelClaim,
    ) -> Result<SessionContext, ResidentProblem> {
        let _entry = self.entry(token)?;
        self.admission.context(token, claim)
    }
    fn entry(&self, token: ResidentToken) -> Result<&Entry, ResidentProblem> {
        let entry = self
            .entries
            .get(&token.address.session)
            .ok_or(ResidentProblem::Missing)?;
        if entry.token != token {
            return Err(ResidentProblem::StaleEpoch);
        }
        Ok(entry)
    }
    fn entry_mut(&mut self, token: ResidentToken) -> Result<&mut Entry, ResidentProblem> {
        let _current = self.entry(token)?;
        self.entries
            .get_mut(&token.address.session)
            .ok_or(ResidentProblem::Missing)
    }
    /// Local view events need their complete current presentation, including
    /// visibility/focus. They cannot borrow through the globally selected route.
    pub(crate) fn resources(
        &self,
        context: &SessionContext,
    ) -> Result<&SessionResources, ResidentProblem> {
        self.admission.validate_context(context)?;
        Ok(&self.entry(context.resident)?.resources)
    }
    pub(crate) fn resources_mut(
        &mut self,
        context: &SessionContext,
    ) -> Result<&mut SessionResources, ResidentProblem> {
        self.admission.validate_context(context)?;
        Ok(&mut self.entry_mut(context.resident)?.resources)
    }
}

#[cfg(test)]
mod requests_owner_tests;
