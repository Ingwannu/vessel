//! Recursive binary groups give each split a bounded, validated fraction.
use super::{GroupId, LayoutError, MAX_TREE_DEPTH, PaneId, SplitFraction, TabId};
use serde::{Deserialize, Serialize};

/// Orientation of a pane group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    /// Children lie left and right.
    Horizontal,
    /// Children lie above and below.
    Vertical,
}
/// A drop at the center moves; an edge creates a new group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// Before horizontally.
    Left,
    /// After horizontally.
    Right,
    /// Before vertically.
    Top,
    /// After vertically.
    Bottom,
    /// Existing pane, no split.
    Center,
}
/// Pane metadata excludes renderer buffers and resource lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pane {
    pub(super) id: PaneId,
    pub(super) tabs: Vec<TabId>,
    pub(super) focused: Option<TabId>,
}
impl Pane {
    pub(super) fn empty(id: PaneId) -> Self {
        Self {
            id,
            tabs: Vec::new(),
            focused: None,
        }
    }
    /// Stable toolkit identity.
    #[must_use]
    pub const fn id(&self) -> PaneId {
        self.id
    }
    /// Tab strip order.
    #[must_use]
    pub fn tabs(&self) -> &[TabId] {
        &self.tabs
    }
    /// Panel selected inside the pane.
    #[must_use]
    pub const fn focused(&self) -> Option<TabId> {
        self.focused
    }
    pub(super) fn remove(&mut self, tab: TabId) {
        let previous = self.tabs.iter().position(|id| *id == tab);
        self.tabs.retain(|id| *id != tab);
        if self.focused == Some(tab) {
            self.focused = previous
                .and_then(|index| self.tabs.get(index.saturating_sub(1)).copied())
                .or_else(|| self.tabs.first().copied());
        }
    }
}
/// Read-only tree projection for the renderer; mutations go through Layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitNode {
    /// Ordinary pane.
    Pane(Pane),
    /// Binary split; fraction belongs to the first child.
    Group {
        /// Stable resize/drag identity, independent from the selected leaf.
        id: GroupId,
        /// Row or column.
        axis: Axis,
        /// First child's normalized share.
        fraction: SplitFraction,
        /// Stable ordered children.
        children: [Box<SplitNode>; 2],
    },
}
impl SplitNode {
    pub(super) fn pane(&self, id: PaneId) -> Option<&Pane> {
        match self {
            Self::Pane(pane) => (pane.id == id).then_some(pane),
            Self::Group { children, .. } => children.iter().find_map(|node| node.pane(id)),
        }
    }
    pub(super) fn pane_mut(&mut self, id: PaneId) -> Option<&mut Pane> {
        match self {
            Self::Pane(pane) => (pane.id == id).then_some(pane),
            Self::Group { children, .. } => children.iter_mut().find_map(|node| node.pane_mut(id)),
        }
    }
    /// Stable document order supports traversal and narrow projection.
    #[must_use]
    pub fn panes(&self) -> Vec<&Pane> {
        match self {
            Self::Pane(pane) => vec![pane],
            Self::Group { children, .. } => children.iter().flat_map(|node| node.panes()).collect(),
        }
    }
    pub(super) fn split(
        &mut self,
        id: PaneId,
        fresh: PaneId,
        position: Position,
        depth: usize,
    ) -> Result<(), LayoutError> {
        match self {
            Self::Pane(pane) => {
                if pane.id != id {
                    return Err(LayoutError::MissingPane);
                }
                if depth >= MAX_TREE_DEPTH {
                    return Err(LayoutError::TreeTooDeep);
                }
                let axis = match position {
                    Position::Left | Position::Right => Axis::Horizontal,
                    Position::Top | Position::Bottom => Axis::Vertical,
                    Position::Center => return Err(LayoutError::InvalidState),
                };
                let old = Box::new(self.clone());
                let new = Box::new(Self::Pane(Pane::empty(fresh)));
                let children = match position {
                    Position::Left | Position::Top => [new, old],
                    Position::Right | Position::Bottom => [old, new],
                    Position::Center => return Err(LayoutError::InvalidState),
                };
                *self = Self::Group {
                    id: GroupId(fresh.0),
                    axis,
                    fraction: SplitFraction::HALF,
                    children,
                };
                Ok(())
            }
            Self::Group { children, .. } => {
                let child = children
                    .iter_mut()
                    .find(|node| node.pane(id).is_some())
                    .ok_or(LayoutError::MissingPane)?;
                child.split(id, fresh, position, depth.saturating_add(1))
            }
        }
    }
    pub(super) fn without(self, id: PaneId) -> Option<Self> {
        match self {
            Self::Pane(pane) => (pane.id != id).then_some(Self::Pane(pane)),
            Self::Group {
                id: group,
                axis,
                fraction,
                children: [a, b],
            } => match (a.without(id), b.without(id)) {
                (Some(a), Some(b)) => Some(Self::Group {
                    id: group,
                    axis,
                    fraction,
                    children: [Box::new(a), Box::new(b)],
                }),
                (Some(node), None) | (None, Some(node)) => Some(node),
                (None, None) => None,
            },
        }
    }
    /// Resize one stable group, preserving finite named limits.
    ///
    /// # Errors
    /// Refuses nonfinite geometry or a missing group.
    pub fn resize(&mut self, group: GroupId, fraction: f32) -> Result<(), LayoutError> {
        if !fraction.is_finite() {
            return Err(LayoutError::InvalidState);
        }
        match self {
            Self::Pane(_) => Err(LayoutError::MissingPane),
            Self::Group {
                id,
                children,
                fraction: share,
                ..
            } => {
                if *id == group {
                    *share = SplitFraction::clamped(fraction)?;
                    return Ok(());
                }
                for child in children {
                    if child.group_exists(group) {
                        return child.resize(group, fraction);
                    }
                }
                Err(LayoutError::MissingPane)
            }
        }
    }
    fn group_exists(&self, group: GroupId) -> bool {
        match self {
            Self::Pane(_) => false,
            Self::Group { id, children, .. } => {
                *id == group || children.iter().any(|child| child.group_exists(group))
            }
        }
    }
}
