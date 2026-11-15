//! File resources belong to node/root/path, independently of pane or
//! renderer. Leaving a tab retains its outstanding I/O and dirty buffer.

use vessel_proto::workspace::{RelPath, WorkspaceRoot};

use super::{CodePanel, Files, Slot, Viewer, actions};
use crate::shell::Shell;
use vessel_app_view::workspace::file_tree::{TreeRow, tree_rows_with};

/// One-MiB editor buffers require an aggregate retention ceiling.
const MAX_RETAINED_FILES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct FileKey {
    pub(super) machine: String,
    pub(super) root: WorkspaceRoot,
    pub(super) path: RelPath,
}

impl CodePanel {
    pub(super) fn file_key(&self) -> Option<FileKey> {
        let (machine, root) = self.root()?;
        Some(FileKey {
            machine: machine.to_owned(),
            root: *root,
            path: self.files.viewer.as_ref()?.path.clone(),
        })
    }

    pub(super) fn viewer_for(&mut self, key: &FileKey) -> Option<&mut Viewer> {
        if self.file_key().as_ref() == Some(key) {
            self.files.viewer.as_mut()
        } else {
            self.files.retained.get_mut(key)
        }
    }
}

impl Shell {
    /// Pending/dirty content cannot be evicted to make room for a file.
    pub(super) fn park_file(&mut self) -> bool {
        let Some(key) = self.areas.workspace.code.file_key() else {
            return true;
        };
        let full = self.areas.workspace.code.files.retained.len() >= MAX_RETAINED_FILES;
        if full {
            let disposable =
                self.areas
                    .workspace
                    .code
                    .files
                    .retained
                    .iter()
                    .find_map(|(key, viewer)| {
                        let idle = viewer.edit.as_ref().is_none_or(|editing| {
                            !editing.model.modified()
                                && editing.reading.is_none()
                                && editing.writing.is_none()
                        });
                        idle.then(|| key.clone())
                    });
            let Some(disposable) = disposable else {
                self.areas.workspace.code.files.actions.refused =
                    Some(self.words().labels().editor_buffers_full.to_owned());
                return false;
            };
            let _evicted = self.areas.workspace.code.files.retained.remove(&disposable);
        }
        let files = &mut self.areas.workspace.code.files;
        if let Some(viewer) = files.viewer.take() {
            let _previous = files.retained.insert(key, viewer);
        }
        true
    }
}

impl Files {
    fn relist(&mut self) {
        self.rows = tree_rows_with(&self.listings, &self.open, self.options);
        self.list.reset(self.row_count());
    }

    /// The list's rows: the tree's, and the row a name is typed in.
    fn row_count(&self) -> usize {
        let extra = self
            .actions
            .editing
            .as_ref()
            .and_then(|editing| actions::edit_slot(&self.rows, &editing.kind))
            .is_some_and(|(_, replaces)| !replaces);
        self.rows.len().saturating_add(usize::from(extra))
    }

    /// What list row `index` draws: the typed name's row (at its depth),
    /// or the tree's row at an index.
    fn slot(&self, index: usize) -> Slot {
        let Some((at, replaces)) = self
            .actions
            .editing
            .as_ref()
            .and_then(|editing| actions::edit_slot(&self.rows, &editing.kind))
        else {
            return Slot::Row(index);
        };
        if index == at {
            let depth = self.rows.get(at).map_or(0, TreeRow::depth);
            return Slot::Edit(depth);
        }
        if index > at && !replaces {
            return Slot::Row(index.saturating_sub(1));
        }
        Slot::Row(index)
    }
}
