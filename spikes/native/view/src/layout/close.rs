//! Lifecycle-aware close plans never archive a child or kill a terminal locally.
use super::{AgentRole, Layout, LayoutError, PaneId, Tab, TabId, Target};
use serde::{Deserialize, Serialize};
use vessel_proto::{SessionId, TerminalId};

/// Dirty work is owned by its editor, including while the tab is an overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modified {
    /// Safe to drop local presentation.
    Clean,
    /// Must be saved or explicitly discarded by the resource owner.
    Dirty,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum AgentCloseRole {
    Root,
    Child,
    Unknown,
}
impl From<AgentRole> for AgentCloseRole {
    fn from(role: AgentRole) -> Self {
        match role {
            AgentRole::Root => Self::Root,
            AgentRole::Child => Self::Child,
            AgentRole::Unknown => Self::Unknown,
        }
    }
}
/// Work a feature controller must complete before removing presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseEffect {
    /// Passive content (setup close never cancels setup).
    Local,
    /// Root agent follows archive/confirmation lifecycle.
    ArchiveAgent(SessionId),
    /// Hydration must resolve the owner before any archive decision.
    ResolveAgent(SessionId),
    /// Child uses per-client visibility, never archive.
    HideChild(SessionId),
    /// Terminal owner confirms and closes its resource.
    CloseTerminal(TerminalId),
    /// Browser owner closes the node resource and its watch.
    CloseBrowser(super::ResourceId),
}
/// Snapshot returned to a lifecycle owner; a later dirty change blocks completion.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosePlan {
    tab: TabId,
    target: Target,
    effect: CloseEffect,
    modified: Modified,
}
impl ClosePlan {
    pub(super) fn of(tab: &Tab) -> Self {
        let effect = match (&tab.target, tab.role) {
            (Target::Agent(id), AgentCloseRole::Unknown) => CloseEffect::ResolveAgent(*id),
            (Target::Agent(id), AgentCloseRole::Root) => CloseEffect::ArchiveAgent(*id),
            (Target::Agent(id), AgentCloseRole::Child) => CloseEffect::HideChild(*id),
            (Target::Terminal(id), _) => CloseEffect::CloseTerminal(*id),
            (Target::Browser(id), _) => CloseEffect::CloseBrowser(id.clone()),
            (
                Target::NewTab
                | Target::Draft(_)
                | Target::ProviderSubagent { .. }
                | Target::Changes
                | Target::Files
                | Target::Scripts
                | Target::PullRequest
                | Target::File { .. }
                | Target::WorkingDiff
                | Target::Plugin { .. }
                | Target::Setup(_)
                | Target::CommitDiff(_),
                _,
            ) => CloseEffect::Local,
        };
        Self {
            tab: tab.id,
            target: tab.target.clone(),
            effect,
            modified: tab.modified,
        }
    }
    /// Controller routes the exact typed resource to its lifecycle owner.
    #[must_use]
    pub fn effect(&self) -> CloseEffect {
        self.effect.clone()
    }
    /// Confirmation must include dirty work as well as process lifecycle.
    #[must_use]
    pub const fn modified(&self) -> Modified {
        self.modified
    }
    /// Stable id remains valid while an async confirmation is pending.
    #[must_use]
    pub const fn tab(&self) -> TabId {
        self.tab
    }
}
/// Owner's result, explicit enough that a local close cannot imply process success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseApproval {
    /// Passive tab with no resource lifecycle.
    Local,
    /// Resource owner completed or acknowledged its required lifecycle.
    LifecycleSettled,
}
impl Layout {
    /// Complete only after the owner handled lifecycle and unsaved work.
    ///
    /// # Errors
    /// Refuses missing or changed targets, dirty work, or an unsettled lifecycle.
    pub fn close_confirmed(
        &mut self,
        plan: &ClosePlan,
        approval: CloseApproval,
    ) -> Result<(), LayoutError> {
        let current = self.tabs.get(&plan.tab).ok_or(LayoutError::MissingTab)?;
        if current.target != plan.target {
            return Err(LayoutError::InvalidState);
        }
        if current.modified == Modified::Dirty {
            return Err(LayoutError::Modified);
        }
        if plan.effect != CloseEffect::Local && approval != CloseApproval::LifecycleSettled {
            return Err(LayoutError::InvalidState);
        }
        self.detach(plan.tab);
        let _closed = self.tabs.remove(&plan.tab);
        Ok(())
    }
    /// A pane can close only after its tab owners have finished teardown.
    ///
    /// # Errors
    /// Refuses a missing pane or one whose content owners have not completed close.
    pub fn close_empty_pane(&mut self, pane: PaneId) -> Result<(), LayoutError> {
        if pane == PaneId::EXPLORER {
            self.explorer_shown = super::ExplorerVisibility::Hidden;
            return Ok(());
        }
        let source = self.root.pane(pane).ok_or(LayoutError::MissingPane)?;
        if !source.tabs.is_empty() {
            return Err(LayoutError::InvalidState);
        }
        if self.root.panes().len() == 1 {
            return Ok(());
        }
        let next = self
            .root
            .clone()
            .without(pane)
            .ok_or(LayoutError::InvalidState)?;
        self.root = next;
        if self.focused == pane {
            self.focused = self
                .root
                .panes()
                .first()
                .map(|pane| pane.id)
                .ok_or(LayoutError::InvalidState)?;
        }
        if self.side == Some(pane) {
            self.side = None;
        }
        Ok(())
    }
    /// Sheet dismissal refuses every dirty descendant before changing anything.
    ///
    /// # Errors
    /// Refuses a missing overlay or any dirty descendant.
    pub fn dismiss_sheet(&mut self, id: TabId) -> Result<(), LayoutError> {
        let index = self
            .sheets
            .iter()
            .position(|sheet| sheet.tab == id)
            .ok_or(LayoutError::MissingTab)?;
        let descendants = self.sheets.get(index..).ok_or(LayoutError::InvalidState)?;
        if descendants.iter().any(|sheet| {
            self.tabs
                .get(&sheet.tab)
                .is_some_and(|tab| tab.modified == Modified::Dirty)
        }) {
            return Err(LayoutError::Modified);
        }
        let ids: Vec<_> = descendants.iter().map(|sheet| sheet.tab).collect();
        for id in ids {
            // Only passive sheet content is removed here; agents remain owned resources.
            self.detach(id);
            let _closed = self.tabs.remove(&id);
        }
        Ok(())
    }
    /// Promotion preserves id, target and owner-maintained buffer state.
    ///
    /// # Errors
    /// Refuses missing sheets, unsupported promotion or a missing destination.
    pub fn promote_sheet(&mut self, id: TabId) -> Result<(), LayoutError> {
        let sheet = self
            .sheets
            .iter()
            .find(|sheet| sheet.tab == id)
            .ok_or(LayoutError::MissingTab)?;
        if sheet.promotion != super::SheetPromotion::Allowed {
            return Err(LayoutError::UnsupportedHost);
        }
        self.sheets.retain(|sheet| sheet.tab != id);
        self.pane_mut(self.focused)?.tabs.push(id);
        self.select(id)
    }
}
