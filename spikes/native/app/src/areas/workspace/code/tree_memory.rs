//! Restore only known expanded child directories, respecting visibility
//! and the reference depth budget. No speculative whole-repository walk.

use vessel_app_view::machine_book::Name;
use vessel_app_view::workspace::explorer::HiddenFiles;
use vessel_app_view::workspace::explorer_memory::MAX_RESTORE_DEPTH;
use vessel_app_view::workspace::file_tree::Listing;
use vessel_proto::workspace::{EntryKind, RelPath};

use crate::shell::Shell;

impl Shell {
    pub(super) fn restore_file_tree(&mut self) {
        let Some((machine, root)) = self
            .areas
            .workspace
            .code
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        let Ok(machine) = Name::parse(&machine) else {
            return;
        };
        self.areas.workspace.code.files.open = self
            .prefs
            .explorer_expansions
            .expanded(&machine, root)
            .into_iter()
            .collect();
    }

    pub(super) fn remember_file_tree(&mut self) {
        let Some((machine, root)) = self
            .areas
            .workspace
            .code
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        let Ok(machine) = Name::parse(&machine) else {
            return;
        };
        let expanded = self.areas.workspace.code.files.open.iter().cloned();
        if let Err(error) = self
            .prefs
            .explorer_expansions
            .record(&machine, root, expanded)
        {
            self.areas.workspace.code.files.actions.refused = Some(error.to_string());
            return;
        }
        self.command(vessel_app_link::Command::SavePrefs(Box::new(
            self.prefs.clone(),
        )));
    }

    pub(super) fn restore_tree_children(&mut self, parent: &RelPath) {
        let files = &self.areas.workspace.code.files;
        let Some(Listing::Listed(listing)) = files.listings.get(parent) else {
            return;
        };
        let children: Vec<_> = listing
            .entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::Dir)
            .filter(|entry| {
                files.options.hidden == HiddenFiles::Show || !entry.name.starts_with('.')
            })
            .filter_map(|entry| parent.child(&entry.name).ok())
            .filter(|path| {
                files.open.contains(path) && path.as_str().split('/').count() <= MAX_RESTORE_DEPTH
            })
            .collect();
        for child in children {
            self.read_folder(&child);
        }
    }

    pub(super) fn restore_visible_tree(&mut self) {
        let folders: Vec<_> = self
            .areas
            .workspace
            .code
            .files
            .listings
            .keys()
            .cloned()
            .collect();
        for folder in folders {
            self.restore_tree_children(&folder);
        }
    }
}
