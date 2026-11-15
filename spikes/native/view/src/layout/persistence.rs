//! Restored layouts are validated before they can reach interactive operations.
use super::{
    ExplorerVisibility, GroupId, Layout, LayoutError, MAX_TREE_DEPTH, Pane, PaneHost, PaneId,
    SplitNode, Tab, TabId,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// One device-local workspace record; keys never use filesystem paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RememberedLayout {
    format: StorageVersion,
    /// Stable registry identity or explicit old-route bridge.
    pub key: super::LayoutKey,
    /// Validated durable presentation.
    pub layout: Layout,
}

/// First Rust device format; it is not a decoder for Paseo's `AsyncStorage` JSON.
const CURRENT_STORAGE_VERSION: u16 = 1;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
struct StorageVersion(u16);
impl TryFrom<u16> for StorageVersion {
    type Error = LayoutError;
    fn try_from(version: u16) -> Result<Self, Self::Error> {
        if version == CURRENT_STORAGE_VERSION {
            Ok(Self(version))
        } else {
            Err(LayoutError::InvalidState)
        }
    }
}
impl From<StorageVersion> for u16 {
    fn from(version: StorageVersion) -> Self {
        version.0
    }
}
impl RememberedLayout {
    /// Only durable projection enters the preferences file.
    #[must_use]
    pub fn new(key: super::LayoutKey, layout: &Layout) -> Self {
        Self {
            format: StorageVersion(CURRENT_STORAGE_VERSION),
            key,
            layout: layout.persistent(),
        }
    }
}

/// Decode the preferences list before any duplicate key can overwrite retained work.
///
/// # Errors
/// Refuses malformed data, unsupported formats, invalid references or duplicate workspace keys.
pub fn decode_layouts<'de, D: Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<RememberedLayout>, D::Error> {
    let layouts = Vec::<RememberedLayout>::deserialize(decoder)?;
    let mut keys = HashSet::new();
    for entry in &layouts {
        if !keys.insert(&entry.key) {
            return Err(serde::de::Error::custom(LayoutError::InvalidState));
        }
    }
    Ok(layouts)
}

#[derive(Deserialize)]
struct SavedLayout {
    root: SplitNode,
    explorer: Pane,
    explorer_shown: ExplorerVisibility,
    focused: PaneId,
    side: Option<PaneId>,
    tabs: BTreeMap<TabId, Tab>,
    next_tab: u64,
    next_pane: u64,
}
impl<'de> Deserialize<'de> for Layout {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let saved = SavedLayout::deserialize(decoder)?;
        let mut layout = Self {
            root: saved.root,
            explorer: saved.explorer,
            explorer_shown: saved.explorer_shown,
            focused: saved.focused,
            side: saved.side,
            tabs: saved.tabs,
            next_tab: saved.next_tab,
            next_pane: saved.next_pane,
            sheets: Vec::new(),
        };
        layout.validate().map_err(serde::de::Error::custom)?;
        // Device-local role metadata is never lifecycle authority after restart.
        // The hydrated ownership lookup must decide root/child again.
        for tab in layout.tabs.values_mut() {
            tab.role = super::close::AgentCloseRole::Unknown;
        }
        Ok(layout)
    }
}
impl Layout {
    /// Produce only durable presentation; overlay resources and rebased SHAs stay ephemeral.
    #[must_use]
    pub fn persistent(&self) -> Self {
        let mut next = self.clone();
        let ephemeral: Vec<_> = next
            .tabs
            .values()
            .filter(|tab| {
                !tab.target.persistent() || next.sheets.iter().any(|sheet| sheet.tab == tab.id)
            })
            .map(|tab| tab.id)
            .collect();
        for id in ephemeral {
            next.detach(id);
            let _ephemeral = next.tabs.remove(&id);
        }
        next.sheets.clear();
        for tab in next.tabs.values_mut() {
            tab.role = super::close::AgentCloseRole::Unknown;
        }
        next
    }
    fn validate(&self) -> Result<(), LayoutError> {
        let mut groups = BTreeSet::new();
        validate_tree(&self.root, 1, self.next_pane, &mut groups)?;
        if self.root.pane(self.focused).is_none()
            || self.explorer.id != PaneId::EXPLORER
            || self.side.is_some_and(|id| self.root.pane(id).is_none())
        {
            return Err(LayoutError::InvalidState);
        }
        let mut panes = BTreeSet::new();
        let mut assigned = BTreeSet::new();
        for pane in self
            .root
            .panes()
            .into_iter()
            .chain(std::iter::once(&self.explorer))
        {
            if !panes.insert(pane.id) || pane.id.0 >= self.next_pane {
                return Err(LayoutError::InvalidState);
            }
            if pane.focused.is_some_and(|id| !pane.tabs.contains(&id)) {
                return Err(LayoutError::InvalidState);
            }
            let host = if pane.id == PaneId::EXPLORER {
                PaneHost::Explorer
            } else {
                PaneHost::Main
            };
            for id in &pane.tabs {
                let tab = self.tabs.get(id).ok_or(LayoutError::InvalidState)?;
                if !assigned.insert(*id)
                    || tab.id != *id
                    || id.0 >= self.next_tab
                    || !tab.target.supports(host)
                {
                    return Err(LayoutError::InvalidState);
                }
            }
        }
        if assigned.len() != self.tabs.len() {
            return Err(LayoutError::InvalidState);
        }
        let mut resources = Vec::new();
        for tab in self.tabs.values() {
            if !matches!(tab.target, super::Target::NewTab) && resources.contains(&&tab.target) {
                return Err(LayoutError::InvalidState);
            }
            resources.push(&tab.target);
        }
        Ok(())
    }
}
fn validate_tree(
    node: &SplitNode,
    depth: usize,
    next_pane: u64,
    groups: &mut BTreeSet<GroupId>,
) -> Result<(), LayoutError> {
    if depth > MAX_TREE_DEPTH {
        return Err(LayoutError::TreeTooDeep);
    }
    match node {
        SplitNode::Pane(pane) => {
            if pane.id == PaneId::EXPLORER {
                return Err(LayoutError::InvalidState);
            }
            Ok(())
        }
        SplitNode::Group { id, children, .. } => {
            // Resize handles share the split allocator. A restored collision can
            // redirect a gesture to an ancestor or to a future split.
            if id.0 >= next_pane || !groups.insert(*id) {
                return Err(LayoutError::InvalidState);
            }
            for child in children {
                validate_tree(child, depth.saturating_add(1), next_pane, groups)?;
            }
            Ok(())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{AgentRole, LayoutMode, OpenIntent, Placement, Target};
    #[test]
    fn restoring_dangling_focus_or_duplicate_panes_is_refused() {
        let layout = Layout {
            focused: PaneId(999),
            ..Layout::default()
        };
        let encoded = serde_json::to_string(&layout).unwrap();
        assert!(serde_json::from_str::<Layout>(&encoded).is_err());
        let mut layout = Layout::default();
        layout.root = SplitNode::Group {
            id: GroupId(2),
            axis: crate::layout::Axis::Horizontal,
            fraction: super::super::SplitFraction::HALF,
            children: [Box::new(layout.root.clone()), Box::new(layout.root.clone())],
        };
        assert!(serde_json::from_str::<Layout>(&serde_json::to_string(&layout).unwrap()).is_err());
    }
    #[test]
    fn durable_projection_omits_sheets_and_keeps_valid_resource_references() {
        let mut layout = Layout::default();
        let _opened = layout
            .open(
                Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                AgentRole::Root,
                LayoutMode::Compact,
            )
            .unwrap();
        let child = layout
            .open(
                Target::Agent(vessel_proto::SessionId::random()),
                OpenIntent::Reveal,
                Placement::Focused,
                AgentRole::Child,
                LayoutMode::Compact,
            )
            .unwrap();
        let saved = layout.persistent();
        assert!(saved.tab(child).is_none());
        let restored: Layout =
            serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
        assert_eq!(restored, saved);
        assert!(layout.tab(child).is_some());
    }
}

#[cfg(test)]
mod document_tests {
    use super::*;
    fn record() -> RememberedLayout {
        RememberedLayout::new(
            crate::layout::LayoutKey::Session {
                machine: crate::layout::HostAlias::parse("local").unwrap(),
                session: vessel_proto::SessionId::random(),
            },
            &Layout::default(),
        )
    }
    #[test]
    fn duplicate_identity_and_unknown_format_never_overwrite_a_saved_layout() {
        let entry = record();
        let text = serde_json::to_string(&vec![entry.clone(), entry.clone()]).unwrap();
        assert!(decode_layouts(&mut serde_json::Deserializer::from_str(&text)).is_err());
        let valid = serde_json::to_string(&entry).unwrap();
        let unknown = valid.replace(
            &format!("\"format\":{CURRENT_STORAGE_VERSION}"),
            &format!("\"format\":{}", CURRENT_STORAGE_VERSION.saturating_add(1)),
        );
        assert!(serde_json::from_str::<RememberedLayout>(&unknown).is_err());
        assert_eq!(
            serde_json::from_str::<RememberedLayout>(&valid).unwrap(),
            entry
        );
    }
}

#[cfg(test)]
mod role_tests {
    use super::*;
    use crate::layout::{AgentRole, CloseEffect, LayoutMode, OpenIntent, Placement, Target};
    #[test]
    fn persisted_root_cannot_authorize_archive_until_ownership_hydrates_again() {
        let mut layout = Layout::default();
        let session = vessel_proto::SessionId::random();
        let tab = layout
            .open(
                Target::Agent(session),
                OpenIntent::Reveal,
                Placement::Focused,
                AgentRole::Root,
                LayoutMode::SplitPanes,
            )
            .unwrap();
        assert_eq!(
            layout.request_close(tab).unwrap().effect(),
            CloseEffect::ArchiveAgent(session)
        );
        let restored: Layout =
            serde_json::from_str(&serde_json::to_string(&layout).unwrap()).unwrap();
        assert_eq!(
            restored.request_close(tab).unwrap().effect(),
            CloseEffect::ResolveAgent(session)
        );
        assert_eq!(
            layout.persistent().request_close(tab).unwrap().effect(),
            CloseEffect::ResolveAgent(session)
        );
    }
}
