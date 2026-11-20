//! The projects folded in the sidebar's project tree (TRACKER 682).
//!
//! [Decision Log]
//! - Purpose: a folded project stays folded across launches, as Paseo keeps
//!   its collapsed projects per device (`sidebar-collapsed-sections`).
//! - Choice: the native document keeps them, written only while one is
//!   folded, so the document the page and the bridge share is unchanged
//!   until then (as `layout.sidebar_width`). A project is its machine and
//!   its id ([`ProjectKey`]): two machines' copies of a project are two
//!   headers, folded apart.
//! - Kept tidy (the app reviewer's note on TRACKER 682, folded into 813): a
//!   whole project list drops the folds of projects it no longer holds, a
//!   machine renamed in the book keeps its folds under its new name, a
//!   forgotten machine's go ([`ProjectKeyed`]); so the document does not
//!   grow with folds of projects that are gone. The key and the tidying are
//!   shared with the dragged order (TRACKER 811); the document's shape is
//!   unchanged (`{machine, project}` entries).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use vessel_proto::ProjectId;

use super::projects::{ProjectKey, ProjectKeyed};

/// The folded projects.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FoldedProjects(BTreeSet<ProjectKey>);

impl FoldedProjects {
    /// Whether none is folded (the document leaves the field out).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether `project` on `machine` is folded.
    #[must_use]
    pub fn contains(&self, machine: &str, project: ProjectId) -> bool {
        self.0.iter().any(|folded| folded.is(machine, project))
    }

    /// Fold it when open, open it when folded.
    pub fn toggle(&mut self, machine: &str, project: ProjectId) {
        let key = ProjectKey::new(machine, project);
        if !self.0.remove(&key) {
            let _new = self.0.insert(key);
        }
    }
}

impl ProjectKeyed for FoldedProjects {
    fn keep_listed(&mut self, machine: &str, listed: &[ProjectId]) -> bool {
        let before = self.0.len();
        self.0
            .retain(|folded| folded.machine != machine || listed.contains(&folded.project));
        self.0.len() != before
    }

    fn rename_machine(&mut self, from: &str, to: &str) -> bool {
        let (moved, kept): (BTreeSet<_>, BTreeSet<_>) = std::mem::take(&mut self.0)
            .into_iter()
            .partition(|folded| folded.machine == from);
        self.0 = kept;
        let any = !moved.is_empty();
        self.0.extend(
            moved
                .into_iter()
                .map(|folded| ProjectKey::new(to, folded.project)),
        );
        any
    }
}
