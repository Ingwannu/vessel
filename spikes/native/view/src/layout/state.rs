//! Atomic local operations. Rejected drag/open leaves the previous layout intact.
use super::{
    ClosePlan, LayoutError, Modified, Pane, PaneHost, PaneId, Position, SplitNode, TabId, Target,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Platform capability is separate from the compact breakpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// One selected pane with transient supporting sheets.
    Compact,
    /// Wide touch surface without drag/split capability.
    SinglePane,
    /// Desktop supports recursive groups and cross-pane drag.
    SplitPanes,
}
/// Passive reveals never imply resource creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenIntent {
    /// A user-created instance (launchers are always unique).
    New,
    /// Select an existing resource where the user put it.
    Reveal,
    /// Reconcile without stealing focus.
    Background,
}
/// Placement preference does not move existing resources; explicit placement does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Use the active ordinary pane.
    Focused,
    /// Move an existing target too.
    Pane(PaneId),
    /// Only new targets use this pane.
    Prefer(PaneId),
    /// Reuse or create a side pane.
    Side,
    /// Retained singleton dock outside the split tree.
    Explorer,
}
/// Unknown ownership behaves as a root during hydration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRole {
    /// Hydrated workspace root.
    Root,
    /// Layout treats this as a root, but close must resolve ownership first.
    Unknown,
    /// Known workspace child can open in a compact sheet.
    Child,
}
/// Metadata retained independently of the toolkit view lifetime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tab {
    pub(super) id: TabId,
    pub(super) target: Target,
    pub(super) modified: Modified,
    pub(super) role: super::close::AgentCloseRole,
}
impl Tab {
    /// Stable identity survives pane moves and sheet promotion.
    #[must_use]
    pub const fn id(&self) -> TabId {
        self.id
    }
    /// Panel-owned resource identity.
    #[must_use]
    pub const fn target(&self) -> &Target {
        &self.target
    }
    /// A close must check the latest dirty state.
    #[must_use]
    pub const fn modified(&self) -> Modified {
        self.modified
    }
}
/// Sheet references the same tab identity as promotion, never a copied buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    /// Stable panel identity.
    pub tab: TabId,
    /// Origin permits focus restoration after nested overlays.
    pub parent: Option<TabId>,
    /// Only known child agents offer promotion.
    pub promotion: SheetPromotion,
}
/// Compact sheet manifest's user action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetPromotion {
    /// Move this tab into the ordinary pane.
    Allowed,
    /// File/provider child remains an overlay.
    Unavailable,
}
/// Serializable layout excludes transient sheets and platform mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Layout {
    pub(super) root: SplitNode,
    pub(super) explorer: Pane,
    pub(super) explorer_shown: ExplorerVisibility,
    pub(super) focused: PaneId,
    pub(super) side: Option<PaneId>,
    pub(super) tabs: BTreeMap<TabId, Tab>,
    pub(super) next_tab: u64,
    pub(super) next_pane: u64,
    #[serde(skip)]
    pub(super) sheets: Vec<Sheet>,
}
/// Hiding Explorer preserves its shell and selected content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExplorerVisibility {
    /// Dock can be toggled back with state intact.
    Hidden,
    /// Dock or compact Explorer overlay is visible.
    Shown,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            root: SplitNode::Pane(Pane::empty(PaneId::MAIN)),
            explorer: Pane::empty(PaneId::EXPLORER),
            explorer_shown: ExplorerVisibility::Hidden,
            focused: PaneId::MAIN,
            side: None,
            tabs: BTreeMap::new(),
            next_tab: 0,
            next_pane: 2,
            sheets: Vec::new(),
        }
    }
}
impl Layout {
    /// Renderers borrow a projection, never alter tree internals.
    #[must_use]
    pub const fn root(&self) -> &SplitNode {
        &self.root
    }
    /// Selected ordinary pane (Explorer selection never destroys it).
    #[must_use]
    pub const fn focused_pane(&self) -> PaneId {
        self.focused
    }
    /// Explorer state remains available while hidden.
    #[must_use]
    pub const fn explorer(&self) -> &Pane {
        &self.explorer
    }
    /// Distinguishes dock visibility from the contents' lifetime.
    #[must_use]
    pub const fn explorer_visibility(&self) -> ExplorerVisibility {
        self.explorer_shown
    }
    /// Feature descriptor lookup for a stable tab id.
    #[must_use]
    pub fn tab(&self, id: TabId) -> Option<&Tab> {
        self.tabs.get(&id)
    }
    /// Stable iteration supports tab switchers and reconciliation.
    pub fn tabs(&self) -> impl Iterator<Item = &Tab> {
        self.tabs.values()
    }
    /// Active overlay is last; its parents remain retained beneath it.
    #[must_use]
    pub fn sheets(&self) -> &[Sheet] {
        &self.sheets
    }
    /// All mutations validate on a copy before committing to the live layout.
    ///
    /// # Errors
    /// Refuses invalid hosts, missing panes, dirty nested overlays, excess depth or exhausted identities.
    pub fn open(
        &mut self,
        target: Target,
        intent: OpenIntent,
        placement: Placement,
        role: AgentRole,
        mode: LayoutMode,
    ) -> Result<TabId, LayoutError> {
        let mut next = self.clone();
        let tab = next.open_inner(target, intent, placement, role, mode)?;
        *self = next;
        Ok(tab)
    }
    fn open_inner(
        &mut self,
        target: Target,
        intent: OpenIntent,
        placement: Placement,
        role: AgentRole,
        mode: LayoutMode,
    ) -> Result<TabId, LayoutError> {
        // Restored plugin slots survive, but live placement needs the plugin
        // owner's validated generation/location capability, not static identity.
        if matches!(target, Target::Plugin { .. }) {
            return Err(LayoutError::UnsupportedHost);
        }
        let sheet_policy = super::compact::policy(&target, role);
        let explorer = matches!(target, Target::Changes | Target::Files);
        let id = self.record_tab(target, role)?;
        if intent == OpenIntent::Background && self.sheets.iter().any(|sheet| sheet.tab == id) {
            return Ok(id);
        }
        if mode == LayoutMode::Compact
            && intent != OpenIntent::Background
            && let Some(promotion) = sheet_policy
        {
            self.reveal_sheet(id, promotion)?;
            return Ok(id);
        }
        let pane = if let Some(current) = self.containing(id) {
            match placement {
                Placement::Pane(id) => id,
                Placement::Explorer => PaneId::EXPLORER,
                Placement::Focused | Placement::Prefer(_) | Placement::Side => current,
            }
        } else {
            match placement {
                Placement::Pane(id) | Placement::Prefer(id) => id,
                Placement::Focused => self.focused,
                Placement::Explorer => PaneId::EXPLORER,
                Placement::Side => self.ensure_side(mode)?,
            }
        };
        let pane = if explorer { PaneId::EXPLORER } else { pane };
        if mode != LayoutMode::SplitPanes
            && pane != PaneId::EXPLORER
            && self
                .containing(id)
                .is_some_and(|current| current != pane && current != PaneId::EXPLORER)
        {
            return Err(LayoutError::SplitsUnavailable);
        }
        let host = if pane == PaneId::EXPLORER {
            PaneHost::Explorer
        } else {
            PaneHost::Main
        };
        if !self
            .tabs
            .get(&id)
            .ok_or(LayoutError::MissingTab)?
            .target
            .supports(host)
        {
            return Err(LayoutError::UnsupportedHost);
        }
        if self.pane(pane).is_none() {
            return Err(LayoutError::MissingPane);
        }
        if self.containing(id) != Some(pane) {
            self.detach(id);
            self.pane_mut(pane)?.tabs.push(id);
        }
        if intent != OpenIntent::Background {
            self.select(id)?;
        }
        if pane == PaneId::EXPLORER && intent != OpenIntent::Background {
            self.explorer_shown = ExplorerVisibility::Shown;
        }
        Ok(id)
    }
    fn record_tab(&mut self, target: Target, role: AgentRole) -> Result<TabId, LayoutError> {
        if let Some(tab) = self
            .tabs
            .values_mut()
            .find(|tab| tab.target.same_resource(&target))
        {
            if role != AgentRole::Unknown {
                tab.role = super::close::AgentCloseRole::from(role);
            }
            return Ok(tab.id);
        }
        let id = TabId(self.next_tab);
        self.next_tab = self.next_tab.checked_add(1).ok_or(LayoutError::Exhausted)?;
        let previous = self.tabs.insert(
            id,
            Tab {
                id,
                target,
                modified: Modified::Clean,
                role: super::close::AgentCloseRole::from(role),
            },
        );
        if previous.is_some() {
            return Err(LayoutError::InvalidState);
        }
        Ok(id)
    }
    pub(super) fn pane(&self, id: PaneId) -> Option<&Pane> {
        if id == PaneId::EXPLORER {
            Some(&self.explorer)
        } else {
            self.root.pane(id)
        }
    }
    pub(super) fn pane_mut(&mut self, id: PaneId) -> Result<&mut Pane, LayoutError> {
        if id == PaneId::EXPLORER {
            Ok(&mut self.explorer)
        } else {
            self.root.pane_mut(id).ok_or(LayoutError::MissingPane)
        }
    }
    /// Discover where a revealed resource was placed by its user.
    #[must_use]
    pub fn containing(&self, tab: TabId) -> Option<PaneId> {
        self.root
            .panes()
            .into_iter()
            .chain(std::iter::once(&self.explorer))
            .find(|pane| pane.tabs.contains(&tab))
            .map(|pane| pane.id)
    }
    pub(super) fn detach(&mut self, id: TabId) {
        let panes: Vec<_> = self.root.panes().into_iter().map(|pane| pane.id).collect();
        for pane in panes {
            if let Some(pane) = self.root.pane_mut(pane) {
                pane.remove(id);
            }
        }
        self.explorer.remove(id);
        self.sheets.retain(|sheet| sheet.tab != id);
    }
    /// Select a tab without resource RPCs or re-creating panel state.
    ///
    /// # Errors
    /// Refuses a tab that is not assigned to a surviving pane.
    pub fn select(&mut self, id: TabId) -> Result<(), LayoutError> {
        let pane = self.containing(id).ok_or(LayoutError::MissingTab)?;
        self.pane_mut(pane)?.focused = Some(id);
        if pane != PaneId::EXPLORER {
            self.focused = pane;
        }
        Ok(())
    }
    /// User focus follows a surviving ordinary pane.
    ///
    /// # Errors
    /// Refuses a missing ordinary pane. Explorer does not replace ordinary focus.
    pub fn focus(&mut self, pane: PaneId) -> Result<(), LayoutError> {
        if self.root.pane(pane).is_none() {
            return Err(LayoutError::MissingPane);
        }
        self.focused = pane;
        Ok(())
    }
    /// Empty split is safe; feature launchers fill it only when available.
    ///
    /// # Errors
    /// Refuses disabled splitting, missing panes, excess depth or exhausted identities.
    pub fn split(
        &mut self,
        pane: PaneId,
        position: Position,
        mode: LayoutMode,
    ) -> Result<PaneId, LayoutError> {
        if mode != LayoutMode::SplitPanes {
            return Err(LayoutError::SplitsUnavailable);
        }
        let fresh = PaneId(self.next_pane);
        let next = self
            .next_pane
            .checked_add(1)
            .ok_or(LayoutError::Exhausted)?;
        // One Explorer bookkeeping level is reserved even though it renders separately.
        self.root.split(pane, fresh, position, 1)?;
        self.next_pane = next;
        self.focused = fresh;
        Ok(fresh)
    }
    fn ensure_side(&mut self, mode: LayoutMode) -> Result<PaneId, LayoutError> {
        if let Some(side) = self.side.filter(|id| self.root.pane(*id).is_some()) {
            return Ok(side);
        }
        let side = self.split(self.focused, Position::Right, mode)?;
        self.side = Some(side);
        Ok(side)
    }
    /// Explorer hides without dropping dirty editors or its selected tab.
    pub const fn show_explorer(&mut self, visibility: ExplorerVisibility) {
        self.explorer_shown = visibility;
    }
    /// Buffers publish state; layout cannot infer save success itself.
    ///
    /// # Errors
    /// Refuses a tab that no longer exists.
    pub fn set_modified(&mut self, id: TabId, modified: Modified) -> Result<(), LayoutError> {
        self.tabs
            .get_mut(&id)
            .ok_or(LayoutError::MissingTab)?
            .modified = modified;
        Ok(())
    }
    /// A close plan names effects for lifecycle owners, without applying them.
    ///
    /// # Errors
    /// Refuses a tab that no longer exists.
    pub fn request_close(&self, id: TabId) -> Result<ClosePlan, LayoutError> {
        self.tab(id)
            .ok_or(LayoutError::MissingTab)
            .map(ClosePlan::of)
    }
}
