//! Editor effects target the buffer that produced them, even after a tab
//! or workspace switch. Correlation is assigned before queue submission.

use std::time::Duration;

use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::{Ask, FileAsk, Save};
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

    pub(super) fn run_step(&mut self, step: Step, cx: &mut Context<'_, Self>) {
        if let Some(key) = self.areas.workspace.code.file_key() {
            self.run_step_for(key, step, cx);
        }
    }

    pub(super) fn run_step_for(&mut self, key: FileKey, step: Step, cx: &mut Context<'_, Self>) {
        match step {
            Step::None => {}
            Step::Write { .. } | Step::Read { .. } => self.send_step_for(&key, step),
            Step::Autosave => self.set_autosave_for(key, cx),
            Step::StopAutosave => {
                if let Some(editing) = self.editing_for(&key) {
                    editing.autosave = editing.autosave.wrapping_add(1);
                }
            }
        }
        cx.notify();
    }

    fn next_file_ticket(&mut self) -> Option<FileTicket> {
        match self.areas.workspace.code.tickets.issue() {
            Ok(ticket) => Some(ticket),
            Err(error) => {
                self.areas.workspace.code.files.actions.refused = Some(error.to_string());
                None
            }
        }
    }

    fn send_step_for(&mut self, key: &FileKey, step: Step) {
        let ask = match step {
            Step::Write {
                text,
                base,
                sequence,
            } => {
                let Some(ticket) = self.next_file_ticket() else {
                    return;
                };
                let Some(editing) = self.editing_for(key) else {
                    return;
                };
                editing.writing = Some((ticket, sequence));
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
        self.ask(&key.machine, Ask::File(ask));
    }

    fn read_editor_for(&mut self, key: &FileKey, sequence: u64) {
        let Some(ticket) = self.next_file_ticket() else {
            return;
        };
        let Some(editing) = self.editing_for(key) else {
            return;
        };
        editing.reading = Some((ticket, sequence));
        self.ask(
            &key.machine,
            Ask::File(FileAsk::EditorRead {
                ask: whole_read(key.root, key.path.clone()),
                ticket,
            }),
        );
    }

    pub(super) fn send_editor_read(&mut self, root: WorkspaceRoot, path: RelPath, sequence: u64) {
        let Some(machine) = self
            .areas
            .workspace
            .code
            .root()
            .map(|(on, _)| on.to_owned())
        else {
            return;
        };
        self.read_editor_for(
            &FileKey {
                machine,
                root,
                path,
            },
            sequence,
        );
    }

    /// Leaving a file requests one serialized flush; its retained model
    /// receives the result and schedules any edit made during that write.
    pub(super) fn flush_file(&mut self) {
        let Some(key) = self.areas.workspace.code.file_key() else {
            return;
        };
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
                let Some(editing) = shell
                    .editing_for(&key)
                    .filter(|editing| editing.autosave == wait)
                else {
                    return;
                };
                let step = editing.model.save();
                shell.run_step_for(key, step, cx);
            });
        })
        .detach();
    }
}
