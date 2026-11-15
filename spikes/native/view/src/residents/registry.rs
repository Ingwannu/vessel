//! Sole admission/focus table; app-owned resources are never cloned or swapped.
use super::identity::{Admissions, ResourceOwner};
use super::{
    FollowGeneration, HoldReason, MAX_RESIDENT_SESSIONS, OwnerReadiness, PanelClaim, PanelFocus,
    RegistryId, ResidentEpoch, ResidentProblem, ResidentToken, SessionAddress, SessionContext,
    Visibility,
};
use crate::layout::PaneId;
use std::collections::BTreeMap;
use std::time::Duration;
use vessel_proto::SessionId;

#[derive(Debug)]
pub(super) struct ClaimState {
    pub pane: PaneId,
    pub visibility: Visibility,
}
#[derive(Debug)]
pub(super) struct Resident {
    pub token: ResidentToken,
    pub claims: BTreeMap<PanelClaim, ClaimState>,
    pub owners: BTreeMap<ResourceOwner, OwnerReadiness>,
    pub follow: FollowGeneration,
    pub attached: bool,
    pub operations: Admissions,
    pub next_operation: u64,
    pub unclaimed_at: Option<Duration>,
}
/// One per window. It tracks authority; app state owns the actual resources.
#[derive(Debug)]
pub struct Residents {
    pub(super) registry: RegistryId,
    pub(super) next_epoch: u64,
    pub(super) entries: BTreeMap<SessionId, Resident>,
    pub(super) focused: Option<(SessionId, PanelClaim)>,
}
impl Default for Residents {
    fn default() -> Self {
        Self::new()
    }
}
impl Residents {
    /// Start a new window lifetime; never restore callback tokens from disk.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry: RegistryId::random(),
            next_epoch: 0,
            entries: BTreeMap::new(),
            focused: None,
        }
    }
    /// Claim resources before rendering; identical claims are idempotent.
    ///
    /// # Errors
    /// Refuses conflicting nodes, capacity or exhausted epoch space atomically.
    pub fn claim(
        &mut self,
        address: SessionAddress,
        claim: PanelClaim,
        pane: PaneId,
    ) -> Result<ResidentToken, ResidentProblem> {
        if self.entries.iter().any(|(session, entry)| {
            *session != address.session && entry.claims.contains_key(&claim)
        }) {
            return Err(ResidentProblem::OwnerConflict);
        }
        if let Some(entry) = self.entries.get_mut(&address.session) {
            if entry.token.address != address {
                return Err(ResidentProblem::OwnerConflict);
            }
            let _previous = entry.claims.entry(claim).or_insert(ClaimState {
                pane,
                visibility: Visibility::RetainedHidden,
            });
            entry.unclaimed_at = None;
            return Ok(entry.token);
        }
        if self.entries.len() >= MAX_RESIDENT_SESSIONS {
            return Err(ResidentProblem::Capacity);
        }
        let epoch = self
            .next_epoch
            .checked_add(1)
            .ok_or(ResidentProblem::Exhausted)?;
        let token = ResidentToken {
            registry: self.registry,
            address,
            epoch: ResidentEpoch(epoch),
        };
        let _previous = self.entries.insert(
            address.session,
            Resident {
                token,
                claims: BTreeMap::from([(
                    claim,
                    ClaimState {
                        pane,
                        visibility: Visibility::RetainedHidden,
                    },
                )]),
                owners: ResourceOwner::ALL
                    .into_iter()
                    .map(|owner| (owner, OwnerReadiness::Hold(HoldReason::Loading)))
                    .collect(),
                follow: FollowGeneration(0),
                attached: false,
                operations: Admissions::new(),
                next_operation: 0,
                unclaimed_at: None,
            },
        );
        self.next_epoch = epoch;
        Ok(token)
    }
    pub(super) fn entry(&self, token: ResidentToken) -> Result<&Resident, ResidentProblem> {
        if token.registry != self.registry {
            return Err(ResidentProblem::StaleEpoch);
        }
        let entry = self
            .entries
            .get(&token.address.session)
            .ok_or(ResidentProblem::Missing)?;
        if entry.token != token {
            return Err(ResidentProblem::StaleEpoch);
        }
        Ok(entry)
    }
    pub(super) fn entry_mut(
        &mut self,
        token: ResidentToken,
    ) -> Result<&mut Resident, ResidentProblem> {
        let _current = self.entry(token)?;
        self.entries
            .get_mut(&token.address.session)
            .ok_or(ResidentProblem::Missing)
    }
    /// Resolve current presentation; old pane/focus metadata is never authority.
    ///
    /// # Errors
    /// Refuses a stale resource or a removed presentation claim.
    pub fn context(
        &self,
        token: ResidentToken,
        claim: &PanelClaim,
    ) -> Result<SessionContext, ResidentProblem> {
        let state = self
            .entry(token)?
            .claims
            .get(claim)
            .ok_or(ResidentProblem::StaleEpoch)?;
        let focus = if self
            .focused
            .as_ref()
            .is_some_and(|(id, focused)| *id == token.address.session && focused == claim)
        {
            PanelFocus::Focused
        } else {
            PanelFocus::Background
        };
        Ok(SessionContext {
            resident: token,
            claim: claim.clone(),
            pane: state.pane,
            visibility: state.visibility,
            focus,
        })
    }
    /// Scoped resource access must recheck every captured view context.
    ///
    /// # Errors
    /// Refuses changed pane, visibility, focus, claim or resource lifetime.
    pub fn validate_context(&self, context: &SessionContext) -> Result<(), ResidentProblem> {
        if self.context(context.resident, &context.claim)? != *context {
            return Err(ResidentProblem::StaleEpoch);
        }
        Ok(())
    }
    /// Update placement without recreating resources; hiding releases keyboard focus.
    ///
    /// # Errors
    /// Refuses stale resources or missing claims.
    pub fn present(
        &mut self,
        token: ResidentToken,
        claim: &PanelClaim,
        pane: PaneId,
        visibility: Visibility,
    ) -> Result<(), ResidentProblem> {
        let state = self
            .entry_mut(token)?
            .claims
            .get_mut(claim)
            .ok_or(ResidentProblem::StaleEpoch)?;
        state.pane = pane;
        state.visibility = visibility;
        if visibility == Visibility::RetainedHidden
            && self
                .focused
                .as_ref()
                .is_some_and(|(id, focused)| *id == token.address.session && focused == claim)
        {
            self.focused = None;
        }
        Ok(())
    }
    /// Give one visible claim keyboard ownership, without authorizing network work.
    ///
    /// # Errors
    /// Hidden, missing or stale claims cannot acquire focus.
    pub fn focus(
        &mut self,
        token: ResidentToken,
        claim: &PanelClaim,
    ) -> Result<(), ResidentProblem> {
        if self.context(token, claim)?.visibility != Visibility::Visible {
            return Err(ResidentProblem::Held);
        }
        self.focused = Some((token.address.session, claim.clone()));
        Ok(())
    }
    /// Current keyboard owner is resolved at dispatch time by the sole interceptor.
    #[must_use]
    pub fn focused(&self) -> Option<SessionContext> {
        let (session, claim) = self.focused.as_ref()?;
        let token = self.entries.get(session)?.token;
        self.context(token, claim).ok()
    }
    /// Owners replace only their own readiness after retaining a concrete snapshot.
    ///
    /// # Errors
    /// Refuses stale resource lifetimes.
    pub fn readiness(
        &mut self,
        token: ResidentToken,
        owner: ResourceOwner,
        readiness: OwnerReadiness,
    ) -> Result<(), ResidentProblem> {
        let _previous = self.entry_mut(token)?.owners.insert(owner, readiness);
        Ok(())
    }
    /// Release presentation after its close plan settles; no backend effect is sent.
    ///
    /// # Errors
    /// Refuses stale resources and unknown claims.
    pub fn release_claim(
        &mut self,
        token: ResidentToken,
        claim: &PanelClaim,
        now: Duration,
    ) -> Result<(), ResidentProblem> {
        let entry = self.entry_mut(token)?;
        if entry.claims.remove(claim).is_none() {
            return Err(ResidentProblem::StaleEpoch);
        }
        if entry.claims.is_empty() {
            entry.unclaimed_at = Some(now);
        }
        if self
            .focused
            .as_ref()
            .is_some_and(|(id, focused)| *id == token.address.session && focused == claim)
        {
            self.focused = None;
        }
        Ok(())
    }
    /// Dehydrate only after all owners transfer clean snapshots. Epoch retirement
    /// precedes app task cancellation; existing layout claims remain on disk.
    ///
    /// # Errors
    /// Refuses visible/focused resources, owner holds or outstanding operations.
    pub fn retire_clean(&mut self, token: ResidentToken) -> Result<(), ResidentProblem> {
        let entry = self.entry(token)?;
        if entry
            .claims
            .values()
            .any(|claim| claim.visibility == Visibility::Visible)
            || self
                .focused
                .as_ref()
                .is_some_and(|(id, _)| *id == token.address.session)
            || entry
                .owners
                .values()
                .any(|ready| *ready != OwnerReadiness::Ready)
            || !entry.operations.is_empty()
        {
            return Err(ResidentProblem::Held);
        }
        let _removed = self.entries.remove(&token.address.session);
        Ok(())
    }
    /// Clean unclaimed expiry uses caller-supplied monotonic elapsed time.
    #[must_use]
    pub fn expired(&self, token: ResidentToken, now: Duration) -> bool {
        self.entry(token)
            .ok()
            .and_then(|entry| entry.unclaimed_at)
            .is_some_and(|released| now.saturating_sub(released) >= super::UNCLAIMED_RESIDENT_TTL)
    }
}
