//! Drag/reorder and keyboard operations share the same atomic move path.
use super::close::Keep;
use super::{Layout, LayoutError, LayoutMode, PaneId, Position, TabId};

/// Logical direction, independent of toolkit key names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Previous horizontal neighbor.
    Left,
    /// Next horizontal neighbor.
    Right,
    /// Previous vertical neighbor.
    Up,
    /// Next vertical neighbor.
    Down,
}
/// Index in a pane's strip, clamped at the end after validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabIndex(usize);
impl TabIndex {
    /// A UI list index becomes a typed insertion position.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}
impl Layout {
    /// Reordering never creates resource ids or changes the selected panel.
    ///
    /// # Errors
    /// Refuses a tab or pane that no longer exists.
    pub fn reorder(&mut self, tab: TabId, index: TabIndex) -> Result<(), LayoutError> {
        if matches!(
            self.tab(tab).ok_or(LayoutError::MissingTab)?.target(),
            super::Target::Plugin { .. }
        ) {
            return Err(LayoutError::UnsupportedHost);
        }
        let pane = self.containing(tab).ok_or(LayoutError::MissingTab)?;
        let pane = self.pane_mut(pane)?;
        pane.tabs.retain(|id| *id != tab);
        let at = index.0.min(pane.tabs.len());
        pane.tabs.insert(at, tab);
        Ok(())
    }
    /// Pointer and keyboard drops validate against identical host/mode rules.
    ///
    /// # Errors
    /// Refuses unsupported hosts, disabled splits, missing resources or excess nesting.
    pub fn drop_tab(
        &mut self,
        tab: TabId,
        destination: PaneId,
        position: Position,
        mode: LayoutMode,
    ) -> Result<(), LayoutError> {
        let mut next = self.clone();
        let target = next.tab(tab).ok_or(LayoutError::MissingTab)?.target.clone();
        if matches!(target, super::Target::Plugin { .. }) {
            return Err(LayoutError::UnsupportedHost);
        }
        if mode != LayoutMode::SplitPanes && next.containing(tab) != Some(destination) {
            return Err(LayoutError::SplitsUnavailable);
        }
        let pane = if position == Position::Center {
            destination
        } else {
            next.split(destination, position, mode)?
        };
        let host = if pane == PaneId::EXPLORER {
            super::PaneHost::Explorer
        } else {
            super::PaneHost::Main
        };
        if !target.supports(host) {
            return Err(LayoutError::UnsupportedHost);
        }
        if next.pane(pane).is_none() {
            return Err(LayoutError::MissingPane);
        }
        let source = next.containing(tab);
        next.detach(tab);
        next.pane_mut(pane)?.tabs.push(tab);
        next.select(tab)?;
        // Paseo's move keeps the source only when the tab stays in it or
        // crosses into or out of Explorer (`preserveEmptyPaneId`).
        if let Some(source) = source
            && source != destination
            && pane != PaneId::EXPLORER
        {
            next.collapse_emptied(source, Keep::No)?;
        }
        *self = next;
        Ok(())
    }
    /// Relative strip traversal wraps and respects the pane's explicit order.
    ///
    /// # Errors
    /// Refuses an inconsistent focused pane or tab reference.
    pub fn step_tab(&mut self, direction: Direction) -> Result<Option<TabId>, LayoutError> {
        let pane = self.pane(self.focused).ok_or(LayoutError::MissingPane)?;
        if pane.tabs.is_empty() {
            return Ok(None);
        }
        let current = pane
            .focused
            .and_then(|id| pane.tabs.iter().position(|candidate| *candidate == id));
        let last = pane.tabs.len().saturating_sub(1);
        let index = match (current, direction) {
            (None, _) => 0,
            (Some(0), Direction::Left | Direction::Up) => last,
            (Some(at), Direction::Left | Direction::Up) => at.saturating_sub(1),
            (Some(at), Direction::Right | Direction::Down) if at == last => 0,
            (Some(at), Direction::Right | Direction::Down) => at.saturating_add(1),
        };
        let tab = pane
            .tabs
            .get(index)
            .copied()
            .ok_or(LayoutError::InvalidState)?;
        self.select(tab)?;
        Ok(Some(tab))
    }
    /// A validated digit shortcut selects a pane-local tab without reordering.
    ///
    /// # Errors
    /// Refuses an inconsistent focused pane. An absent index is a no-op.
    pub fn jump_tab(&mut self, index: TabIndex) -> Result<Option<TabId>, LayoutError> {
        let tab = self
            .pane(self.focused)
            .ok_or(LayoutError::MissingPane)?
            .tabs
            .get(index.0)
            .copied();
        if let Some(tab) = tab {
            self.select(tab)?;
        }
        Ok(tab)
    }
    /// Parent split resizing is finite and clamped by the shared tree bound.
    ///
    /// # Errors
    /// Refuses nonfinite geometry or a missing group.
    pub fn resize(&mut self, group: super::GroupId, fraction: f32) -> Result<(), LayoutError> {
        self.root.resize(group, fraction)
    }
}

/// Tab strip operations share an index-independent close selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseScope {
    /// This tab only.
    Current,
    /// Preceding siblings in this pane.
    Before,
    /// Following siblings in this pane.
    After,
    /// All siblings except this tab.
    Others,
    /// Every tab in its pane.
    Pane,
}
/// Registry-facing commands contain intent, never raw keyboard strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAction {
    /// Open the available launcher in the focused pane.
    NewTab,
    /// Lifecycle-aware closure of the current selection.
    Close(CloseScope),
    /// Desktop split direction.
    Split(Position),
    /// Focus a neighboring pane.
    Focus(Direction),
    /// Move selected tab to a neighboring pane.
    Move(Direction),
    /// Close after each content owner has settled teardown.
    ClosePane,
    /// Previous/next within the focused pane.
    StepTab(Direction),
    /// Exact pane-local position.
    JumpTab(TabIndex),
    /// Maximize projection without losing tree structure.
    ToggleFocus,
}
impl super::SplitNode {
    fn neighbor(&self, current: PaneId, direction: Direction) -> Option<PaneId> {
        match self {
            Self::Pane(_) => None,
            Self::Group {
                axis,
                children: [a, b],
                ..
            } => {
                let forward = matches!(
                    (axis, direction),
                    (super::Axis::Horizontal, Direction::Right)
                        | (super::Axis::Vertical, Direction::Down)
                );
                let backward = matches!(
                    (axis, direction),
                    (super::Axis::Horizontal, Direction::Left)
                        | (super::Axis::Vertical, Direction::Up)
                );
                if a.pane(current).is_some() {
                    a.neighbor(current, direction).or_else(|| {
                        forward
                            .then(|| b.panes().first().map(|pane| pane.id()))
                            .flatten()
                    })
                } else if b.pane(current).is_some() {
                    b.neighbor(current, direction).or_else(|| {
                        backward
                            .then(|| a.panes().last().map(|pane| pane.id()))
                            .flatten()
                    })
                } else {
                    None
                }
            }
        }
    }
}
impl Layout {
    /// Structural neighbors follow actual split axes, not raw tab registration order.
    ///
    /// # Errors
    /// Refuses an inconsistent neighbor reference. A missing neighbor is a no-op.
    pub fn focus_neighbor(&mut self, direction: Direction) -> Result<(), LayoutError> {
        if let Some(next) = self.root.neighbor(self.focused, direction) {
            self.focus(next)?;
        }
        Ok(())
    }
    /// Keyboard move uses the same validated drop operation as pointer drag.
    ///
    /// # Errors
    /// Refuses invalid drag destinations or a surface without split capability.
    pub fn move_neighbor(
        &mut self,
        direction: Direction,
        mode: LayoutMode,
    ) -> Result<(), LayoutError> {
        let tab = self.pane(self.focused).and_then(|pane| pane.focused);
        if let (Some(tab), Some(next)) = (tab, self.root.neighbor(self.focused, direction)) {
            self.drop_tab(tab, next, Position::Center, mode)?;
        }
        Ok(())
    }
    /// Bulk UI can count lifecycle effects before asking for confirmation.
    ///
    /// # Errors
    /// Refuses a missing tab or pane instead of selecting another workspace.
    pub fn close_plans(
        &self,
        tab: TabId,
        scope: CloseScope,
    ) -> Result<Vec<super::ClosePlan>, LayoutError> {
        let pane = self
            .containing(tab)
            .and_then(|pane| self.pane(pane))
            .ok_or(LayoutError::MissingTab)?;
        let position = pane
            .tabs
            .iter()
            .position(|id| *id == tab)
            .ok_or(LayoutError::MissingTab)?;
        pane.tabs
            .iter()
            .enumerate()
            .filter(|(index, id)| match scope {
                CloseScope::Current => **id == tab,
                CloseScope::Before => *index < position,
                CloseScope::After => *index > position,
                CloseScope::Others => **id != tab,
                CloseScope::Pane => true,
            })
            .map(|(_, id)| self.request_close(*id))
            .collect()
    }
}

/// Tab-over-tab insertion uses the pointer's half of the destination chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabInsertion {
    /// Insert immediately before the destination after removing the source.
    Before,
    /// Insert immediately after the destination after removing the source.
    After,
}
impl Layout {
    /// One path for same-pane reorder and cross-pane tab-over-tab drops.
    ///
    /// # Errors
    /// Refuses stale tabs or any destination rejected by the pane drag policy.
    pub fn drop_onto_tab(
        &mut self,
        source: TabId,
        target: TabId,
        insertion: TabInsertion,
        mode: LayoutMode,
    ) -> Result<(), LayoutError> {
        if source == target {
            return Ok(());
        }
        let pane = self.containing(target).ok_or(LayoutError::MissingTab)?;
        let mut next = self.clone();
        next.drop_tab(source, pane, Position::Center, mode)?;
        let at = next
            .pane(pane)
            .and_then(|pane| pane.tabs.iter().position(|id| *id == target))
            .ok_or(LayoutError::MissingTab)?;
        let at = match insertion {
            TabInsertion::Before => at,
            TabInsertion::After => at.saturating_add(1),
        };
        next.reorder(source, TabIndex::new(at))?;
        *self = next;
        Ok(())
    }
}
