//! Admission precedes hydration, callbacks and transport launch. No network data
//! can allocate a resident; this staged entry takes an explicit local open intent.
use super::{SessionResidents, SessionResources};
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::layout::{
    AgentRole, HostAlias, Layout, LayoutError, LayoutKey, LayoutMode, LookupProblem, OpenIntent,
    Placement, Target, WorkspaceLookup, session_layout_key,
};
use vessel_app_view::residents::{
    FollowBinding, FollowRetirement, PanelClaim, ResidentProblem, SessionAddress, SessionContext,
    Visibility,
};
use vessel_proto::SessionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionOpeningProblem {
    Lookup(LookupProblem),
    Layout(LayoutError),
    Resident(ResidentProblem),
}

/// An original selected connection and authoritative placement, never a later
/// alias lookup. Resolving ownership does not authorize any network operation.
pub(crate) struct SessionOpening {
    connection: ConnectionContext,
    address: SessionAddress,
    workspace: LayoutKey,
}
impl SessionOpening {
    pub(crate) fn resolve(
        lookup: &impl WorkspaceLookup,
        connection: ConnectionContext,
        session: SessionId,
    ) -> Result<Self, SessionOpeningProblem> {
        let alias = HostAlias::parse(connection.machine().as_str())
            .map_err(SessionOpeningProblem::Layout)?;
        let address = SessionAddress {
            node: connection.node(),
            session,
        };
        let workspace = session_layout_key(lookup, address.node, session, alias)
            .map_err(SessionOpeningProblem::Lookup)?;
        Ok(Self {
            connection,
            address,
            workspace,
        })
    }
    fn validate_placement(
        &self,
        lookup: &impl WorkspaceLookup,
    ) -> Result<(), SessionOpeningProblem> {
        let current = Self::resolve(lookup, self.connection.clone(), self.address.session)?;
        if current.workspace != self.workspace {
            return Err(SessionOpeningProblem::Resident(
                ResidentProblem::OwnerConflict,
            ));
        }
        Ok(())
    }
}

/// Carries the exact intent into the link. A delivery capability and this binding
/// must both validate at receive; this value is not transport permission.
pub(crate) struct SessionAdmission {
    pub(crate) connection: ConnectionContext,
    pub(crate) context: SessionContext,
    pub(crate) binding: FollowBinding,
}
impl SessionResidents {
    fn admit_single(
        &mut self,
        opening: &SessionOpening,
        layout: &mut Layout,
        mode: LayoutMode,
        build: impl FnOnce(&SessionContext) -> SessionResources,
        retired: impl FnOnce(&mut SessionResources, &FollowRetirement),
    ) -> Result<SessionAdmission, SessionOpeningProblem> {
        let (next, claim, pane) = self.plan_open(opening, layout, mode)?;
        let token = self
            .claim_session(opening.address, &claim, pane, build)
            .map_err(SessionOpeningProblem::Resident)?;
        let binding = self
            .begin_follow(token, retired)
            .map_err(SessionOpeningProblem::Resident)?;
        self.present(token, &claim, pane, Visibility::Visible)
            .map_err(SessionOpeningProblem::Resident)?;
        self.focus(token, &claim)
            .map_err(SessionOpeningProblem::Resident)?;
        let context = self
            .context(token, &claim)
            .map_err(SessionOpeningProblem::Resident)?;
        self.entry_mut(token)
            .map_err(SessionOpeningProblem::Resident)?
            .connection = Some(opening.connection.clone());
        *layout = next;
        Ok(SessionAdmission {
            connection: opening.connection.clone(),
            context,
            binding,
        })
    }

    fn plan_open(
        &self,
        opening: &SessionOpening,
        layout: &Layout,
        mode: LayoutMode,
    ) -> Result<(Layout, PanelClaim, vessel_app_view::layout::PaneId), SessionOpeningProblem> {
        // The first cut never implicitly evicts even clean input. Explicit
        // all-owner snapshot/close must release the bundle before another opens.
        if self
            .entries
            .keys()
            .any(|session| *session != opening.address.session)
        {
            return Err(SessionOpeningProblem::Resident(ResidentProblem::Capacity));
        }
        let mut next = layout.clone();
        let tab = next
            .open(
                Target::Agent(opening.address.session),
                OpenIntent::Reveal,
                Placement::Focused,
                AgentRole::Unknown,
                mode,
            )
            .map_err(SessionOpeningProblem::Layout)?;
        // A root reveal beneath a retained sheet cannot claim visible input.
        // The sheet owner must dismiss it through its existing dirty guard.
        if !next.sheets().is_empty() {
            return Err(SessionOpeningProblem::Resident(ResidentProblem::Held));
        }
        let pane = next
            .containing(tab)
            .ok_or(SessionOpeningProblem::Layout(LayoutError::MissingPane))?;
        let claim = PanelClaim {
            workspace: opening.workspace.clone(),
            tab,
        };
        if let Some(entry) = self.entries.get(&opening.address.session) {
            // Refuse cross-node identity and alternate claims before factory,
            // focus changes or retiring a still-valid old attempt.
            if entry.token.address != opening.address {
                return Err(SessionOpeningProblem::Resident(
                    ResidentProblem::OwnerConflict,
                ));
            }
            let _existing = self
                .context(entry.token, &claim)
                .map_err(SessionOpeningProblem::Resident)?;
        }
        Ok((next, claim, pane))
    }

    /// Independent lease retirement must run even when replacement admission
    /// refuses. Exact original context prevents an old notice retiring a successor.
    /// The caller must observe actual shared lease retirement. A follow-only
    /// EOF/error/cancel uses `retire_follow` with its exact binding instead.
    pub(crate) fn connection_retired(
        &mut self,
        connection: &ConnectionContext,
        mut retired: impl FnMut(&mut SessionResources, &FollowRetirement),
    ) -> Result<(), ResidentProblem> {
        for entry in self.entries.values_mut() {
            if entry.connection.as_ref() == Some(connection) {
                if let Some(notice) = self.admission.retire_current(entry.token)? {
                    entry.resources.follow_retired(&notice);
                    retired(&mut entry.resources, &notice);
                }
                entry.connection = None;
            }
        }
        Ok(())
    }
}

impl Shell {
    /// Sole staged minter uses the real book's tab, then publishes its layout
    /// only after resource admission. Product conversion must remove globals.
    pub(crate) fn admit_session_open(
        &mut self,
        opening: &SessionOpening,
        lookup: &impl WorkspaceLookup,
        build: impl FnOnce(&SessionContext, &mut Window, &mut Context<'_, Self>) -> SessionResources,
        retired: impl FnOnce(&mut SessionResources, &FollowRetirement),
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<SessionAdmission, SessionOpeningProblem> {
        // A deferred user intent must not keep authority from an old membership
        // snapshot. Refuse changed placement rather than retargeting its tab.
        opening.validate_placement(lookup)?;
        let before = super::super::registry::retirement::Before::layout(self, &opening.workspace);
        super::super::initialized(self);
        let mut layout = self
            .areas
            .layout
            .book
            .borrow()
            .as_ref()
            .and_then(|book| book.get(&opening.workspace))
            .cloned()
            .unwrap_or_default();
        let admission = self.areas.session_residents.admit_single(
            opening,
            &mut layout,
            super::super::mode(window),
            |context| build(context, window, cx),
            retired,
        )?;
        let _previous = self
            .areas
            .layout
            .book
            .borrow_mut()
            .as_mut()
            .ok_or(SessionOpeningProblem::Layout(LayoutError::InvalidState))?
            .insert(opening.workspace.clone(), layout);
        if self
            .areas
            .layout
            .maximized
            .as_ref()
            .is_some_and(|(key, pane)| key == &opening.workspace && *pane != admission.context.pane)
        {
            self.areas.layout.maximized = None;
        }
        self.layout_retired(
            before,
            super::super::registry::retirement::Source::Local,
            cx,
        );
        self.layout_result(Ok(()), cx);
        Ok(admission)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod presentation_tests;
