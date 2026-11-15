//! Prepare once on fleet/expansion changes. Rendering only reads prepared rows.

use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use vessel_proto::SessionRef;

/// Bound graph preparation rather than silently dropping excess sessions.
pub const MAX_MANAGED_SNAPSHOTS: usize = 16_384;
/// Corrupt/cyclic/deeper graphs become Unknown instead of hiding their rows.
pub const MAX_TREE_DEPTH: usize = 64;

/// UI hydration confidence, distinct from immutable creator/audit information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// A capable authoritative lookup resolved current parent; None includes detach.
    Known(Option<SessionRef>),
    /// Unsupported, missing, stale or conflicting current-parent information.
    Unknown,
}
/// Active children exclude archive intent as well as completed archival.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Visible active session, whatever its process/turn status.
    Active,
    /// Server archive state.
    Archived,
    /// Local accepted archive operation; do not offer it again.
    PendingArchive,
}
/// Missing hosts/relationships must not turn a visible count into a fleet total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeCoverage {
    /// Every source and current relation was hydrated.
    Complete,
    /// Available rows only; consumers disclose that additional rows may exist.
    Partial,
}
/// W is W11's actual WorkspaceId at the caller, preserved without an alias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedSnapshot<W> {
    /// Full node and managed-session identity, even across book aliases.
    pub session: SessionRef,
    /// Current relation from an authoritative capable lookup.
    pub relation: Relation,
    /// Registry-owned workspace membership; no folder/title inference.
    pub workspace: W,
    /// Actual immutable creation time for oldest-first child ordering.
    pub created_at: DateTime<Utc>,
    /// Archive and pending intent are separate from provider local hide.
    pub visibility: Visibility,
}
/// Bad input is a typed refusal, not a successful empty tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TreeProblem {
    /// Caller can keep its flat view while fetching a bounded source snapshot.
    #[error("managed session snapshot exceeds its preparation limit")]
    TooMany,
    /// Conflicting aliases must be hydrated once rather than arbitrarily selected.
    #[error("conflicting managed session snapshots: {0:?}")]
    Ambiguous(SessionRef),
}
/// Indentation is a view unit, unrelated to the node's authorization depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeDepth(usize);
impl TreeDepth {
    /// Number of visible same-workspace ancestors.
    #[must_use]
    pub const fn level(self) -> usize {
        self.0
    }
}
/// A prepared row; labels/status still come from the caller's canonical card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeRow {
    /// Opening this target uses full node identity and the shared pane target.
    pub session: SessionRef,
    /// Prepared indentation, never inferred while painting.
    pub depth: TreeDepth,
}
/// Shared jump result for the sidebar and command center.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reveal<W> {
    /// Actual registry workspace to activate.
    pub workspace: W,
    /// Expand these same-workspace ancestors from root to parent.
    pub ancestors: Vec<SessionRef>,
    /// Target to open/focus after expansion.
    pub target: SessionRef,
}
/// One prepared source drives every consumer; no separate title/cwd tree.
#[derive(Debug)]
pub struct ManagedTree<W> {
    entries: Vec<ManagedSnapshot<W>>,
    indices: HashMap<SessionRef, usize>,
    children: HashMap<SessionRef, Vec<SessionRef>>,
    coverage: TreeCoverage,
}

impl<W: Clone + Eq> ManagedTree<W> {
    /// Deduplicate full identities and reject conflicting authoritative snapshots.
    /// # Errors
    /// Source exceeds the cap or aliases disagree about one managed session.
    pub fn new(
        input: Vec<ManagedSnapshot<W>>,
        mut coverage: TreeCoverage,
    ) -> Result<Self, TreeProblem> {
        if input.len() > MAX_MANAGED_SNAPSHOTS {
            return Err(TreeProblem::TooMany);
        }
        let mut entries: Vec<ManagedSnapshot<W>> = Vec::new();
        let mut indices = HashMap::new();
        for entry in input {
            if let Some(previous) = indices
                .get(&entry.session)
                .and_then(|index| entries.get(*index))
            {
                if previous != &entry {
                    return Err(TreeProblem::Ambiguous(entry.session));
                }
            } else {
                let _new = indices.insert(entry.session, entries.len());
                entries.push(entry);
            }
        }
        entries.sort_by_key(|entry| (entry.created_at, entry.session.node, entry.session.session));
        indices = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.session, index))
            .collect();
        let mut tree = Self {
            entries,
            indices,
            children: HashMap::new(),
            coverage,
        };
        let relations: Vec<_> = tree
            .entries
            .iter()
            .map(|entry| tree.safe_relation(entry.session))
            .collect();
        for (entry, relation) in tree.entries.iter_mut().zip(relations) {
            entry.relation = relation;
            if relation == Relation::Unknown {
                coverage = TreeCoverage::Partial;
            }
            if entry.visibility == Visibility::Active
                && let Relation::Known(Some(parent)) = relation
            {
                tree.children.entry(parent).or_default().push(entry.session);
            }
        }
        tree.coverage = coverage;
        Ok(tree)
    }

    /// Only actual observed active children, oldest first, across all workspaces.
    #[must_use]
    pub fn children(&self, parent: SessionRef) -> &[SessionRef] {
        self.children.get(&parent).map_or(&[], Vec::as_slice)
    }
    /// Unknown is not a root assertion and must not authorize archive/hide.
    #[must_use]
    pub fn parent(&self, session: SessionRef) -> Relation {
        self.entry(session)
            .map_or(Relation::Unknown, |entry| entry.relation)
    }
    /// Consumers can distinguish a fleet total from a partial visible count.
    #[must_use]
    pub const fn coverage(&self) -> TreeCoverage {
        self.coverage
    }

    /// Same-workspace nesting only; a cross-workspace child stays a normal row
    /// in its own workspace. Every active session appears at most once.
    #[must_use]
    pub fn rows(&self, workspace: &W, expanded: &HashSet<SessionRef>) -> Vec<TreeRow> {
        let roots: Vec<_> = self
            .entries
            .iter()
            .rev()
            .filter(|entry| {
                entry.visibility == Visibility::Active
                    && &entry.workspace == workspace
                    && self.visible_parent(entry, workspace).is_none()
            })
            .collect();
        let mut stack: Vec<_> = roots
            .into_iter()
            .rev()
            .map(|entry| TreeRow {
                session: entry.session,
                depth: TreeDepth(0),
            })
            .collect();
        let mut seen = HashSet::new();
        let mut rows = Vec::new();
        while let Some(row) = stack.pop() {
            if !seen.insert(row.session) {
                continue;
            }
            if expanded.contains(&row.session) {
                stack.extend(
                    self.children(row.session)
                        .iter()
                        .rev()
                        .filter_map(|child| self.entry(*child))
                        .filter(|entry| &entry.workspace == workspace)
                        .map(|entry| TreeRow {
                            session: entry.session,
                            depth: TreeDepth(row.depth.0.saturating_add(1)),
                        }),
                );
            }
            rows.push(row);
        }
        rows
    }

    /// Jump preparation uses this same current-parent model and workspace identity.
    #[must_use]
    pub fn reveal(&self, target: SessionRef) -> Option<Reveal<W>> {
        let entry = self
            .entry(target)
            .filter(|entry| entry.visibility == Visibility::Active)?;
        let mut ancestors = Vec::new();
        let mut current = entry;
        while let Some(parent) = self.visible_parent(current, &entry.workspace) {
            ancestors.push(parent.session);
            current = parent;
        }
        ancestors.reverse();
        Some(Reveal {
            workspace: entry.workspace.clone(),
            ancestors,
            target,
        })
    }

    fn entry(&self, session: SessionRef) -> Option<&ManagedSnapshot<W>> {
        self.indices
            .get(&session)
            .and_then(|index| self.entries.get(*index))
    }
    fn visible_parent(
        &self,
        entry: &ManagedSnapshot<W>,
        workspace: &W,
    ) -> Option<&ManagedSnapshot<W>> {
        let Relation::Known(Some(parent)) = entry.relation else {
            return None;
        };
        self.entry(parent).filter(|parent| {
            parent.visibility == Visibility::Active && &parent.workspace == workspace
        })
    }
    fn safe_relation(&self, session: SessionRef) -> Relation {
        let original = self.parent(session);
        let mut seen = HashSet::new();
        let mut current = Some(session);
        while let Some(id) = current {
            if seen.len() >= MAX_TREE_DEPTH || !seen.insert(id) {
                return Relation::Unknown;
            }
            current = match self.parent(id) {
                Relation::Known(parent) => parent,
                Relation::Unknown => None,
            };
        }
        original
    }
}

#[cfg(test)]
#[path = "managed_tests.rs"]
mod tests;
