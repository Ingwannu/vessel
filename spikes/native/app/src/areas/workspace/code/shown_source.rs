//! Correlation metadata alongside the ONE Changes summary and diff controller.
//! A stamp is source evidence only; action admission remains with its consumer.
#[cfg(test)]
use super::Tab;
use super::{CodePanel, Summary};
#[cfg(test)]
use crate::shell::Shell;
use std::collections::HashMap;
use vessel_app_link::{DeliveryGuard, workspace::code_source::Request};
use vessel_app_view::{
    connection::ConnectionContext,
    workspace::{changes::ScopeKind, file_scope::FileScope, file_ticket::FileTicket},
};
use vessel_proto::{SessionId, workspace::RelPath};

#[derive(Debug, Clone)]
pub(super) struct RootReceipt {
    pub context: ConnectionContext,
    pub session: SessionId,
    pub ticket: FileTicket,
    pub guard: Option<DeliveryGuard>,
}
impl RootReceipt {
    pub fn current(&self) -> bool {
        self.guard
            .as_ref()
            .is_some_and(|guard| guard.validate(&self.context).is_ok())
    }
}
/// Exact origin of the Ready comparison actually stored by this Changes owner.
#[derive(Debug, Clone)]
pub(crate) struct ShownChangesContext {
    pub(super) root: RootReceipt,
    pub(super) scope: FileScope,
    pub(super) ticket: FileTicket,
    pub(super) comparison: ScopeKind,
    pub(super) guard: DeliveryGuard,
}
impl ShownChangesContext {
    /// Original acquisition facts; this does not admit an operation.
    #[cfg(test)]
    pub(crate) fn connection(&self) -> &ConnectionContext {
        self.scope.connection()
    }
    #[cfg(test)]
    pub(crate) const fn root(&self) -> vessel_proto::workspace::WorkspaceRoot {
        self.scope.root()
    }
    #[cfg(test)]
    pub(crate) const fn comparison(&self) -> ScopeKind {
        self.comparison
    }
    fn live(&self) -> bool {
        self.root.current() && self.guard.validate(self.scope.connection()).is_ok()
    }
}
/// Exact parent comparison and most recently installed page of a drawn diff.
#[derive(Debug, Clone)]
pub(crate) struct ShownDiffContext {
    pub(super) summary: ShownChangesContext,
    pub(super) path: RelPath,
    pub(super) ticket: FileTicket,
    pub(super) guard: DeliveryGuard,
}
impl ShownDiffContext {
    #[cfg(test)]
    pub(crate) fn connection(&self) -> &ConnectionContext {
        self.summary.connection()
    }
    #[cfg(test)]
    pub(crate) fn path(&self) -> &RelPath {
        &self.path
    }
    fn live(&self) -> bool {
        self.summary.live() && self.guard.validate(self.summary.scope.connection()).is_ok()
    }
}
#[derive(Default)]
pub(super) struct Sources {
    pub requested: Option<Request>,
    pub ready: Option<ShownChangesContext>,
    pub diffs: HashMap<RelPath, DiffSource>,
}
pub(super) struct DiffSource {
    pub request: Request,
    pub ready: Option<ShownDiffContext>,
}
impl Sources {
    pub fn invalidate(&mut self) {
        self.requested = None;
        self.ready = None;
        self.diffs.clear();
    }
}
impl CodePanel {
    pub(super) fn source(&self) -> Option<&ShownChangesContext> {
        let ready = self.changes.sources.ready.as_ref()?;
        let target = self.target.as_ref()?;
        let root = target.receipt.as_ref()?;
        (ready.live()
            && ready.comparison == self.changes.scope
            && ready.root.context == root.context
            && ready.root.session == target.session
            && ready.root.ticket == root.ticket
            && self.root() == Some((root.context.machine().as_str(), &ready.scope.root()))
            && matches!(self.changes.summary, Summary::Ready(_)))
        .then_some(ready)
    }
    pub(super) fn diff_source(&self, path: &RelPath) -> Option<&ShownDiffContext> {
        let summary = self.source()?;
        let diff = self.changes.sources.diffs.get(path)?.ready.as_ref()?;
        (diff.live()
            && diff.path == *path
            && matches!(&self.changes.sources.diffs.get(path)?.request, Request::Diff { ticket, .. } if *ticket == diff.ticket)
            && diff.summary.ticket == summary.ticket
            && matches!(
                self.changes.states.get(path),
                Some(vessel_app_view::workspace::changes::FileState::Read(_))
            ))
        .then_some(diff)
    }
}
impl Shell {
    /// Original drawn source, available only from the actual legacy Changes host.
    /// A typed Explorer caller must also validate its real PanelContext. Logical
    /// Workspace roots remain unavailable until their canonical runtime producer.
    #[cfg(test)]
    pub(crate) fn shown_changes_context(&self) -> Option<ShownChangesContext> {
        let panel = &self.areas.workspace.code;
        let shown = panel.source()?;
        let open = self.areas.session.open.as_ref()?;
        (self.route == crate::routes::Route::Session
            && panel.open
            && panel.tab == Tab::Changes
            && open.machine == shown.scope.connection().machine().as_str()
            && open.session == shown.root.session)
            .then(|| shown.clone())
    }
    /// Only a real visible Explorer projection may request typed shown facts.
    /// The current shared frame has no mounted Explorer, so that projection
    /// remains unavailable. This never creates a book entry or resolves roots.
    #[cfg(test)]
    pub(crate) fn shown_changes_at(
        &self,
        panel: &crate::areas::layout::registry::PanelContext<'_>,
        window: &vessel_app_kit::ui::Window,
    ) -> Option<ShownChangesContext> {
        use vessel_app_view::layout::{LayoutKey, PaneId, Target};
        if panel.pane != PaneId::EXPLORER
            || *panel.target != Target::Changes
            || !self.layout_tab_matches(panel.workspace, panel.tab, panel.target)
            || !self.layout_panel_visible(panel.workspace, panel.tab, window)
        {
            return None;
        }
        let shown = self.areas.workspace.code.source()?;
        let LayoutKey::Session { machine, session } = panel.workspace else {
            return None;
        };
        (machine.as_str() == shown.scope.connection().machine().as_str()
            && *session == shown.root.session)
            .then(|| shown.clone())
    }
    /// G1 delegates its ONE Git retirement hook; C5 registers no second owner.
    #[cfg(test)]
    pub(crate) fn code_changes_retired(
        &mut self,
        notice: &crate::areas::layout::registry::retirement::Notice,
    ) {
        use vessel_app_view::layout::{LayoutKey, Target};
        let Some(origin) = self
            .areas
            .workspace
            .code
            .target
            .as_ref()
            .and_then(|target| target.receipt.as_ref())
        else {
            return;
        };
        let LayoutKey::Session { machine, session } = notice.workspace() else {
            return;
        };
        if notice.target() == &Target::Changes
            && machine.as_str() == origin.context.machine().as_str()
            && *session == origin.session
        {
            self.areas.workspace.code.changes.sources.invalidate();
        }
    }
    /// Validate immediately at every callback lookup; counts are never identity.
    #[cfg(test)]
    pub(crate) fn shown_changes_is_current(&self, shown: &ShownChangesContext) -> bool {
        self.shown_changes_context().is_some_and(|current| {
            current.ticket == shown.ticket
                && current.scope == shown.scope
                && current.root.ticket == shown.root.ticket
                && shown.live()
        })
    }
    /// A diff carries the same parent source and the last original page ticket.
    #[cfg(test)]
    pub(crate) fn shown_diff_context(&self, path: &RelPath) -> Option<ShownDiffContext> {
        self.shown_changes_context()?;
        self.areas.workspace.code.diff_source(path).cloned()
    }
    #[cfg(test)]
    pub(crate) fn shown_diff_is_current(&self, shown: &ShownDiffContext) -> bool {
        self.shown_diff_context(&shown.path).is_some_and(|current| {
            current.ticket == shown.ticket
                && self.shown_changes_is_current(&shown.summary)
                && shown.live()
        })
    }
}
