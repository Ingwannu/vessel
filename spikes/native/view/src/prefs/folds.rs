//! The projects folded in the sidebar's project tree (TRACKER 682).
//!
//! [Decision Log]
//! - Purpose: a folded project stays folded across launches, as Paseo keeps
//!   its collapsed projects per device (`sidebar-collapsed-sections`).
//! - Choice: the native document keeps them, written only while one is
//!   folded, so the document the page and the bridge share is unchanged
//!   until then (as `layout.sidebar_width`). A project is its machine and
//!   its id: two machines' copies of a project are two headers, folded
//!   apart.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use vessel_proto::ProjectId;

/// The folded projects.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FoldedProjects(BTreeSet<FoldedProject>);

/// One folded project.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FoldedProject {
    /// Its machine's name.
    pub machine: String,
    /// The project there.
    pub project: ProjectId,
}

impl FoldedProjects {
    /// Whether none is folded (the document leaves the field out).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether `project` on `machine` is folded.
    #[must_use]
    pub fn contains(&self, machine: &str, project: ProjectId) -> bool {
        self.0
            .iter()
            .any(|folded| folded.machine == machine && folded.project == project)
    }

    /// Fold it when open, open it when folded.
    pub fn toggle(&mut self, machine: &str, project: ProjectId) {
        let key = FoldedProject {
            machine: machine.to_owned(),
            project,
        };
        if !self.0.remove(&key) {
            let _new = self.0.insert(key);
        }
    }
}
