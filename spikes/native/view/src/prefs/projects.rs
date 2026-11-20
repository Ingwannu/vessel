//! What the native document keeps per project of the sidebar's project
//! tree: its folds (TRACKER 682, `folds`) and its dragged order (TRACKER
//! 811, `order`).
//!
//! [Decision Log]
//! - Purpose: both name a project the same way and are kept tidy the same
//!   way, so one key and one tidying contract serve both.
//! - Key: a project's machine (by its name in the book) and its id there.
//!   The tree heads a project once per machine (two machines' copies of a
//!   project are two headers), so the machine is part of the key.
//! - Tidying ([`ProjectKeyed`]): a machine's whole project list drops what
//!   it no longer holds; a machine renamed in the book keeps its entries
//!   under the new name; a forgotten machine's entries go. The app runs
//!   each change over every keeper (`Shell::tidy_projects`), so a new
//!   keeper cannot be left out of one of them.

use serde::{Deserialize, Serialize};
use vessel_proto::ProjectId;

/// A project of the tree: its machine's name and its id there.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProjectKey {
    /// Its machine's name.
    pub machine: String,
    /// The project there.
    pub project: ProjectId,
}

impl ProjectKey {
    /// `project` on `machine`.
    #[must_use]
    pub fn new(machine: &str, project: ProjectId) -> Self {
        Self {
            machine: machine.to_owned(),
            project,
        }
    }

    /// Whether it is `project` on `machine`.
    #[must_use]
    pub fn is(&self, machine: &str, project: ProjectId) -> bool {
        self.machine == machine && self.project == project
    }
}

/// Entries kept per project, tidied as the projects and the book change;
/// each change says whether anything changed, so the document is written
/// only then.
pub trait ProjectKeyed {
    /// Keep `machine`'s entries only for the projects it lists now. Call
    /// it with a whole list only: a partial one would drop entries of
    /// projects not read.
    fn keep_listed(&mut self, machine: &str, listed: &[ProjectId]) -> bool;

    /// `from`'s entries are `to`'s (the machine was renamed in the book).
    fn rename_machine(&mut self, from: &str, to: &str) -> bool;

    /// Drop `machine`'s entries (it was forgotten).
    fn forget_machine(&mut self, machine: &str) -> bool {
        self.keep_listed(machine, &[])
    }
}
