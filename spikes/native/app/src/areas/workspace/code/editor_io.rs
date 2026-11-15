//! Editor effects target the buffer that produced them, even after a tab
//! or workspace switch. Correlation is assigned before queue submission.

use std::time::Duration;

use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::{FileAsk, Save};
use vessel_app_view::workspace::editor::{AUTOSAVE_MS, Step};
use vessel_app_view::workspace::file_ticket::FileTicket;
use vessel_proto::workspace::{RelPath, WorkspaceRoot};

use super::buffers::FileKey;
use super::editing::{Editing, whole_read};
use crate::shell::Shell;

impl Shell {
    pub(super) fn editing_for(&mut self, key: &FileKey) -> Option<&mut Editing> {
        self.areas.workspace.code.viewer_for(key)?.edit.as_mut()
    }

    pub(super) fn run_step_for(&mut self, key: &FileKey, step: Step, cx: &mut Context<'_, Self>) {
        self.publish_file_tabs(key, cx);
        match step {
            Step::None => {}
            Step::Write { .. } | Step::Read { .. } => self.send_step_for(key, step),
            Step::Autosave => self.set_autosave_for(key.clone(), cx),
            Step::StopAutosave => {
                if let Some(editing) = self.editing_for(key) {
                    editing.autosave = editing.autosave.wrapping_add(1);
                }
            }
        }
        // Queue refusal can settle synchronously inside an effect. Publish
        // that original outcome too; identical acknowledgements are coalesced.
        self.publish_file_tabs(key, cx);
        cx.notify();
    }

    pub(super) fn next_file_ticket(&mut self) -> Option<FileTicket> {
        match self.areas.workspace.code.tickets.issue() {
            Ok(ticket) => Some(ticket),
            Err(error) => {
                self.areas.workspace.code.files.actions.refused = Some(error.to_string());
                None
            }
        }
    }

    pub(super) fn send_step_for(&mut self, key: &FileKey, step: Step) {
        let ask = match step {
            Step::Write {
                text,
                base,
                sequence,
            } => {
                let Some(ticket) = self.next_file_ticket() else {
                    let sentence = self.words().labels().editor_save_failed.to_owned();
                    if let Some(editing) = self.editing_for(key) {
                        let _settled = editing.model.saved(
                            sequence,
                            vessel_app_view::workspace::editor::Written::Failed(sentence),
                        );
                    }
                    return;
                };
                let Some(scope) = self
                    .areas
                    .workspace
                    .code
                    .viewer_for(key)
                    .map(|viewer| viewer.scope.clone())
                else {
                    return;
                };
                let Some(editing) = self.editing_for(key) else {
                    return;
                };
                editing.writing = Some(super::editing::WriteTicket {
                    ticket,
                    sequence,
                    scope: scope.clone(),
                });
                FileAsk::Save(Save {
                    ticket,
                    root: key.root,
                    path: key.path.clone(),
                    text,
                    base,
                })
            }
            Step::Read { sequence } => {
                self.read_editor_for(key, sequence);
                return;
            }
            Step::None | Step::Autosave | Step::StopAutosave => return,
        };
        self.send_file_for(key, ask);
    }

    fn read_editor_for(&mut self, key: &FileKey, sequence: u64) {
        let Some(ticket) = self.next_file_ticket() else {
            return;
        };
        let Some(editing) = self.editing_for(key) else {
            return;
        };
        editing.reading = Some((ticket, sequence));
        self.send_file_for(
            key,
            FileAsk::EditorRead {
                ask: whole_read(key.root, key.path.clone()),
                ticket,
            },
        );
    }

    pub(super) fn send_editor_read(&mut self, root: WorkspaceRoot, path: RelPath, sequence: u64) {
        let Some(scope) = self
            .areas
            .workspace
            .code
            .scope()
            .filter(|scope| scope.root() == root)
        else {
            return;
        };
        let key = FileKey::new(&scope, path);
        self.read_editor_for(&key, sequence);
    }

    /// Leaving a file requests one serialized flush; its retained model
    /// receives the result and schedules any edit made during that write.
    pub(super) fn flush_file(&mut self) {
        let Some(key) = self.areas.workspace.code.file_key() else {
            return;
        };
        if self.areas.workspace.code.file_effect_held(&key) {
            return;
        }
        let Some(editing) = self.editing_for(&key) else {
            return;
        };
        let step = editing.model.flush();
        self.send_step_for(&key, step);
    }

    fn set_autosave_for(&mut self, key: FileKey, cx: &mut Context<'_, Self>) {
        let Some(editing) = self.editing_for(&key) else {
            return;
        };
        editing.autosave = editing.autosave.wrapping_add(1);
        let wait = editing.autosave;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(AUTOSAVE_MS))
                .await;
            let _gone = this.update(cx, |shell, cx| {
                if shell.areas.workspace.code.file_effect_held(&key) {
                    return;
                }
                let Some(editing) = shell
                    .editing_for(&key)
                    .filter(|editing| editing.autosave == wait)
                else {
                    return;
                };
                let step = editing.model.save();
                shell.run_step_for(&key, step, cx);
            });
        })
        .detach();
    }
}

/// The renderer boundary converts canonical one-based source coordinates
/// once, leaving navigation out of the retained file identity.
pub(super) fn reveal_location(
    editor: &vessel_app_kit::code_editor::CodeEditor,
    location: vessel_app_view::workspace::file_location::FileLocation,
    window: &mut vessel_app_kit::ui::Window,
    cx: &mut Context<'_, Shell>,
) {
    use vessel_app_kit::code_editor::SourcePoint;
    let range = location.range();
    let point = |position: vessel_proto::lsp::Position| SourcePoint {
        line: position.line.get().saturating_sub(1),
        character: position.column.get().saturating_sub(1),
    };
    editor.reveal_range(&(point(range.start)..point(range.end)), window, cx);
}
