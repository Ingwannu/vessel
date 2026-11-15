//! Captured-scope viewer acquisition/rebinding, using the one retained owner map.
use super::{Shown, Viewer};
use crate::shell::Shell;
use vessel_app_kit::VirtualList;
use vessel_app_link::workspace::FileAsk;
use vessel_app_view::workspace::file_location::FileLocation;
use vessel_proto::workspace::{ReadFile, RelPath, WorkspaceRoot};
impl Shell {
    /// Show `path`: edited in place at a desktop's width (read whole, in
    /// one page of at most 1 MiB), read-only at a phone's (Paseo's rule).
    pub(super) fn show_file(
        &mut self,
        machine: &str,
        root: WorkspaceRoot,
        path: RelPath,
        location: Option<FileLocation>,
    ) {
        let workspace_dir = self
            .areas
            .workspace
            .code
            .cwd()
            .map(std::path::Path::to_path_buf);
        let Some(scope) = self.areas.workspace.code.scope().filter(|scope| {
            scope.connection().machine().as_str() == machine && scope.root() == root
        }) else {
            return;
        };
        let key = super::buffers::FileKey::new(&scope, path.clone());
        let files = &mut self.areas.workspace.code.files;
        if let Some(mut viewer) = files.retained.remove(&key) {
            if location.is_some() {
                viewer.goto = location;
                if let Some(preview) = &mut viewer.markdown {
                    preview.reveal_source();
                }
            }
            let retry_read = matches!(viewer.text, Shown::Failed(_))
                || viewer.edit.as_ref().is_some_and(|editing| {
                    editing.model.status()
                        == vessel_app_view::workspace::editor::EditorStatus::Failed
                        || editing.model.write_state()
                            == Some(vessel_app_view::workspace::editor::WriteState::Ended)
                });
            let rebound = viewer.scope != scope;
            if rebound {
                viewer.watch = super::observation::Watcher::default();
                viewer.scope = scope;
                viewer.reading = None;
                viewer.intelligence = super::intelligence::State::Idle;
                if let Some(editing) = &mut viewer.edit {
                    editing.reading = None;
                    editing.autosave = editing.autosave.wrapping_add(1);
                    editing.model.retire_observations();
                }
            }
            viewer.watch.restart();
            files.viewer = Some(viewer);
            if rebound || retry_read {
                let step = files
                    .viewer
                    .as_mut()
                    .and_then(|viewer| viewer.edit.as_mut())
                    .map(|editing| {
                        let step = editing.model.reconcile();
                        if matches!(step, vessel_app_view::workspace::editor::Step::None)
                            && editing.editor.is_none()
                            && editing.model.write_state().is_none()
                        {
                            vessel_app_view::workspace::editor::Step::Read {
                                sequence: editing.model.reading(),
                            }
                        } else {
                            step
                        }
                    });
                if let Some(step) = step {
                    self.send_step_for(&key, step);
                } else {
                    self.read_viewer(
                        machine,
                        ReadFile {
                            root,
                            path,
                            offset: 0,
                            limit: None,
                        },
                    );
                }
            }
            self.begin_file_watch(&key);
            return;
        }
        let editable = !files.narrow.get();
        files.viewer = Some(Viewer {
            watch: super::observation::Watcher::default(),
            scope,
            markdown: super::markdown::Preview::for_path(&path, location),
            reading: None,
            workspace_dir,
            intelligence: super::intelligence::State::Idle,
            path: path.clone(),
            text: Shown::Loading,
            list: VirtualList::new(false),
            goto: location,
            chosen: None,
            edit: editable.then(super::editing::Editing::new),
        });
        if editable {
            let sequence = files
                .viewer
                .as_ref()
                .and_then(|viewer| viewer.edit.as_ref())
                .map(|edit| edit.model.first_read());
            if let Some(sequence) = sequence {
                self.send_editor_read(root, path, sequence);
            }
            self.begin_file_watch(&key);
            return;
        }
        let read = ReadFile {
            root,
            path,
            offset: 0,
            limit: None,
        };
        self.read_viewer(machine, read);
        self.begin_file_watch(&key);
    }

    pub(super) fn read_viewer(&mut self, _machine: &str, ask: ReadFile) {
        let Some(key) = self
            .areas
            .workspace
            .code
            .file_key()
            .filter(|key| key.root == ask.root && key.path == ask.path)
        else {
            return;
        };
        self.read_viewer_for(&key, ask);
    }
    pub(super) fn read_viewer_for(&mut self, key: &super::buffers::FileKey, ask: ReadFile) {
        let Some(ticket) = self.next_file_ticket() else {
            return;
        };
        let Some(viewer) = self.areas.workspace.code.viewer_for(key) else {
            return;
        };
        viewer.reading = Some(ticket);
        self.send_file_for(key, FileAsk::Read { ask, ticket });
    }
}
