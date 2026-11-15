//! Bounded, validated expansion state persisted with existing app prefs.
//! Roots identify authorized physical folders; no cwd-derived workspace id
//! or second project registry is introduced here.

use crate::machine_book::Name;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use vessel_proto::workspace::{RelPath, WorkspaceRoot};

/// The reference restores expanded directories only to this depth.
pub const MAX_RESTORE_DEPTH: usize = 5;
/// Bound the size of the local preference document across roots.
const MAX_REMEMBERED_ROOTS: usize = 64;
/// Bound restoration work per root independently of directory entry counts.
const MAX_EXPANDED_PATHS: usize = 256;

/// Preference corruption must not silently become an empty overwrite.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExpansionError {
    /// A name is outside the node book's grammar.
    #[error("an explorer host name is invalid")]
    Host,
    /// Too many physical roots in one preference document.
    #[error("explorer history holds at most {MAX_REMEMBERED_ROOTS} roots")]
    Roots,
    /// A root asks to restore unbounded filesystem work.
    #[error("a file tree keeps at most {MAX_EXPANDED_PATHS} expanded directories")]
    Paths,
    /// Duplicate keys make restore order ambiguous.
    #[error("explorer history contains a duplicate root")]
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
struct SavedHost(String);

impl TryFrom<String> for SavedHost {
    type Error = ExpansionError;
    fn try_from(text: String) -> Result<Self, Self::Error> {
        let name = Name::parse(&text).map_err(|_invalid| ExpansionError::Host)?;
        Ok(Self(name.as_str().to_owned()))
    }
}

impl From<SavedHost> for String {
    fn from(host: SavedHost) -> Self {
        host.0
    }
}

/// Serialized input shape. Private fields ensure callers use the bounded
/// `ExplorerMemory::record` path rather than constructing unchecked state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RememberedTree {
    host: SavedHost,
    root: WorkspaceRoot,
    expanded: Vec<RelPath>,
}

/// Global preferences hold a bounded list of per-root expansion choices.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "Vec<RememberedTree>", into = "Vec<RememberedTree>")]
pub struct ExplorerMemory(Vec<RememberedTree>);

impl TryFrom<Vec<RememberedTree>> for ExplorerMemory {
    type Error = ExpansionError;
    fn try_from(trees: Vec<RememberedTree>) -> Result<Self, Self::Error> {
        if trees.len() > MAX_REMEMBERED_ROOTS {
            return Err(ExpansionError::Roots);
        }
        let mut seen = HashSet::new();
        for tree in &trees {
            if tree.expanded.len() > MAX_EXPANDED_PATHS {
                return Err(ExpansionError::Paths);
            }
            if !seen.insert((&tree.host, tree.root)) {
                return Err(ExpansionError::Duplicate);
            }
        }
        Ok(Self(trees))
    }
}

impl From<ExplorerMemory> for Vec<RememberedTree> {
    fn from(memory: ExplorerMemory) -> Self {
        memory.0
    }
}

impl ExplorerMemory {
    /// Paths eligible for automatic restoration, preserving manual deep
    /// choices on disk without triggering an unbounded startup walk.
    #[must_use]
    pub fn expanded(&self, host: &Name, root: WorkspaceRoot) -> Vec<RelPath> {
        self.0
            .iter()
            .find(|tree| tree.host.0 == host.as_str() && tree.root == root)
            .map(|tree| {
                tree.expanded
                    .iter()
                    .filter(|path| depth(path) <= MAX_RESTORE_DEPTH)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Update one root, moving it to the recently used end. Only inactive
    /// tree preferences are evicted; this never owns or drops file buffers.
    ///
    /// # Errors
    /// [`ExpansionError::Paths`] leaves the previous durable state intact.
    pub fn record(
        &mut self,
        host: &Name,
        root: WorkspaceRoot,
        expanded: impl IntoIterator<Item = RelPath>,
    ) -> Result<(), ExpansionError> {
        let mut paths: Vec<_> = expanded
            .into_iter()
            .filter(|path| !path.is_root())
            .take(MAX_EXPANDED_PATHS.saturating_add(1))
            .collect();
        if paths.len() > MAX_EXPANDED_PATHS {
            return Err(ExpansionError::Paths);
        }
        paths.sort();
        paths.dedup();
        self.0
            .retain(|tree| !(tree.host.0 == host.as_str() && tree.root == root));
        if self.0.len() >= MAX_REMEMBERED_ROOTS {
            let _oldest = self.0.remove(0);
        }
        self.0.push(RememberedTree {
            host: SavedHost(host.as_str().to_owned()),
            root,
            expanded: paths,
        });
        Ok(())
    }
}

fn depth(path: &RelPath) -> usize {
    if path.is_root() {
        0
    } else {
        path.as_str().split('/').count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::ProjectId;

    #[test]
    fn restores_only_reference_depth_and_preserves_other_roots() {
        let host = Name::parse("node").unwrap();
        let root = WorkspaceRoot::Project {
            project: ProjectId::random(),
        };
        let other = WorkspaceRoot::Project {
            project: ProjectId::random(),
        };
        let shallow: RelPath = "a/b".parse().unwrap();
        let deep: RelPath = "a/b/c/d/e/f".parse().unwrap();
        let mut memory = ExplorerMemory::default();
        memory.record(&host, root, [shallow.clone(), deep]).unwrap();
        assert_eq!(memory.expanded(&host, root), [shallow]);
        assert!(memory.expanded(&host, other).is_empty());
        assert_eq!(
            serde_json::from_str::<ExplorerMemory>(&serde_json::to_string(&memory).unwrap())
                .unwrap(),
            memory
        );
    }

    #[test]
    fn oversized_restore_is_rejected_and_previous_state_survives() {
        let host = Name::parse("node").unwrap();
        let root = WorkspaceRoot::Project {
            project: ProjectId::random(),
        };
        let mut memory = ExplorerMemory::default();
        memory
            .record(&host, root, ["kept".parse().unwrap()])
            .unwrap();
        let before = memory.clone();
        let paths = (0..=MAX_EXPANDED_PATHS).map(|n| format!("directory-{n}").parse().unwrap());
        assert_eq!(
            memory.record(&host, root, paths),
            Err(ExpansionError::Paths)
        );
        assert_eq!(memory, before);
    }
}
