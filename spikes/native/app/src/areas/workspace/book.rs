//! The node book edited (track E2's Settings › Machines, after the edit
//! succeeded): this area's state is keyed by a machine's name in the book,
//! so a rename moves it to the new name and a forget clears this machine's
//! screens. Nothing is stopped or deleted on the machine itself.
//!
//! [Decision Log]
//! - Purpose: after a rename the projects, the composer's place, a
//!   worktree start and the code panel go on under the new name; after a
//!   forget nothing of this area shows or waits on that machine.
//! - Constraint: E2 owns the book and its one edit type
//!   (`view::machine_book::Edit`); this area does not copy it. A start's
//!   steps already sent cannot be taken back: a session the machine was
//!   already asked to create may still be created (it shows on the board
//!   when the machine is paired again).
//! - Choice: one hook taking E2's edit; the link's follows are ended by
//!   the link itself (`Hub::book_applied`, in the same task as the book's
//!   write); here a renamed machine's projects are listed again under the
//!   new name, which follows its clones anew.
//! - Cost: a rename reads the machine's projects once more.

use std::collections::HashMap;
use std::hash::Hash;

use vessel_app_kit::ui::Context;
use vessel_app_view::machine_book::{Edit, Name};

use crate::shell::Shell;

/// `map`'s entry for `from`, moved to `to` (an entry already at `to` is
/// replaced: the book refuses a rename onto a name it has).
pub(super) fn rekey<V>(map: &mut HashMap<String, V>, from: &str, to: &str) {
    if let Some(value) = map.remove(from) {
        let _was = map.insert(to.to_owned(), value);
    }
}

/// Every entry of `map` on `from`, moved to `to`.
pub(super) fn rekey_pairs<K: Eq + Hash + Clone, V>(
    map: &mut HashMap<(String, K), V>,
    from: &str,
    to: &str,
) {
    let moved: Vec<(String, K)> = map
        .keys()
        .filter(|(machine, _)| machine == from)
        .cloned()
        .collect();
    for key in moved {
        if let Some(value) = map.remove(&key) {
            let _was = map.insert((to.to_owned(), key.1), value);
        }
    }
}

/// Every entry of `map` on `machine`, gone.
pub(super) fn forget_pairs<K, V>(map: &mut HashMap<(String, K), V>, machine: &str) {
    map.retain(|(on, _), _| on != machine);
}

impl Shell {
    /// Track E2's hook, called once after a book edit succeeded.
    pub(crate) fn workspace_book_applied(&mut self, edit: &Edit, cx: &mut Context<'_, Self>) {
        self.layout_book_applied(edit, cx);
        match edit {
            Edit::Rename { from, to } => self.workspace_renamed(from, to),
            Edit::Forget(name) => self.workspace_forgot(name),
            Edit::MakeDefault(_) => {}
        }
        cx.notify();
    }

    fn workspace_renamed(&mut self, from: &Name, to: &Name) {
        self.areas.workspace.registry.forget(from);
        self.project_host_detached(from.as_str());
        self.areas.workspace.project_reads.detach(from);
        let (from, to) = (from.as_str(), to.as_str());
        let workspace = &mut self.areas.workspace;
        let listed = workspace.projects.contains_key(from);
        rekey(&mut workspace.projects, from, to);
        workspace.place.machine_renamed(from, to);
        if let Some(start) = workspace
            .start
            .as_mut()
            .filter(|start| start.machine == from)
        {
            to.clone_into(&mut start.machine);
        }
        workspace.code.machine_renamed(from, to);
        if listed {
            self.read_projects(to);
        }
    }

    fn workspace_forgot(&mut self, name: &Name) {
        self.areas.workspace.registry.forget(name);
        self.project_host_detached(name.as_str());
        self.areas.workspace.project_reads.detach(name);
        let name = name.as_str();
        let workspace = &mut self.areas.workspace;
        let _was = workspace.projects.remove(name);
        workspace.place.machine_forgot(name);
        workspace.page.detail = None;
        workspace.code.machine_forgot(name);
        // A start there: what was not sent yet is not sent (the answers'
        // channel closes, the walk ends at its next step); the draft comes
        // back with its text.
        if workspace
            .start
            .as_ref()
            .is_some_and(|start| start.machine == name)
        {
            workspace.start = None;
            self.start_failed();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{forget_pairs, rekey, rekey_pairs};

    #[test]
    fn a_rename_moves_the_machine_s_entries_and_leaves_the_others() {
        let mut lists = HashMap::from([("laptop".to_owned(), 1), ("build-server".to_owned(), 2)]);
        rekey(&mut lists, "laptop", "work-laptop");
        assert_eq!(lists.get("work-laptop"), Some(&1));
        assert_eq!(lists.get("laptop"), None);
        assert_eq!(lists.get("build-server"), Some(&2));

        let mut branches = HashMap::from([
            (("laptop".to_owned(), 'a'), 1),
            (("laptop".to_owned(), 'b'), 2),
            (("build-server".to_owned(), 'a'), 3),
        ]);
        rekey_pairs(&mut branches, "laptop", "work-laptop");
        assert_eq!(branches.get(&("work-laptop".to_owned(), 'a')), Some(&1));
        assert_eq!(branches.get(&("work-laptop".to_owned(), 'b')), Some(&2));
        assert_eq!(branches.get(&("build-server".to_owned(), 'a')), Some(&3));
        assert_eq!(branches.len(), 3);
    }

    #[test]
    fn a_forget_clears_the_machine_s_entries_only() {
        let mut branches = HashMap::from([
            (("laptop".to_owned(), 'a'), 1),
            (("build-server".to_owned(), 'a'), 3),
        ]);
        forget_pairs(&mut branches, "laptop");
        assert_eq!(branches.len(), 1);
        assert!(branches.contains_key(&("build-server".to_owned(), 'a')));
        let mut lists = HashMap::from([("laptop".to_owned(), 1)]);
        rekey(&mut lists, "missing", "other");
        assert_eq!(
            lists.len(),
            1,
            "a rename of a machine with nothing here is nothing"
        );
    }
}
