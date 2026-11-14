//! A file edited in the Files tab (Paseo's Source panel,
//! `file-panels/source/panel.tsx` `EditableTextFile`): the editor model's
//! steps carried out (a guarded save, a read, the 800 ms autosave), the
//! node's answers fed back, Overwrite, Reload (asked first when the buffer
//! has changes), and the write made when the file is left.
//!
//! [Decision Log]
//! - Purpose: edit a file of the session's project where it is shown.
//! - Constraint: Paseo edits a file of at most 1 MiB on a desktop and
//!   shows every file read-only on a phone and any larger file read-only.
//!   The node's largest page is 1 MiB, so an editable file is read whole
//!   in one page; a file whose answer says there is more stays read-only.
//! - Choice: the model (`view::workspace::editor`) decides, this file does
//!   what it asks; the editor control is made on the next frame after the
//!   first read (answers come without a window), and a reload's text is put
//!   in the control the same way.
//! - Cost: the toolbar's line count walks the text on each frame the panel
//!   draws (a 1 MiB file: well under a millisecond).

use std::time::Duration;

use vessel_app_kit::code_editor::CodeEditor;
use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::workspace::{Ask, FileAsk, Save};
use vessel_app_view::workspace::editor::{
    AUTOSAVE_MS, EditorModel, MAX_EDITABLE_BYTES, Read, Step, Written, wraps,
};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::save::Saved;
use vessel_app_view::workspace::syntax::language_of;
use vessel_proto::ErrorCode;
use vessel_proto::workspace::{FileContent, FileRead, ReadFile, RelPath, WorkspaceRoot};

use super::line_syntax::ViewSyntax;
use crate::shell::Shell;

/// A file being edited.
pub(super) struct Editing {
    pub(super) model: EditorModel,
    /// Made on the frame after the first read (it needs the window).
    pub(super) editor: Option<CodeEditor>,
    /// The caret's line and column and the selection's button.
    pub(super) status: Option<vessel_app_kit::ui::Entity<super::editor_status::Status>>,
    /// The model's text is newer than the control's (a reload).
    pub(super) stale: bool,
    /// Bumped by every autosave set or stopped; a timer checks it.
    autosave: u64,
    /// Reload's question, made when first asked.
    pub(super) reload: Option<DialogState>,
}

impl Editing {
    pub(super) const fn new() -> Self {
        Self {
            model: EditorModel::new(),
            editor: None,
            status: None,
            stale: false,
            autosave: 0,
            reload: None,
        }
    }
}

/// The read an editor asks for: the whole file in one page of at most
/// the editable size.
pub(super) fn whole_read(root: WorkspaceRoot, path: RelPath) -> ReadFile {
    ReadFile {
        root,
        path,
        offset: 0,
        limit: u32::try_from(MAX_EDITABLE_BYTES).ok(),
    }
}

impl Shell {
    fn editing(&mut self) -> Option<&mut Editing> {
        self.areas
            .workspace
            .code
            .files
            .viewer
            .as_mut()?
            .edit
            .as_mut()
    }

    /// Carry out what the model asked.
    fn run_step(&mut self, step: Step, cx: &mut Context<'_, Self>) {
        match step {
            Step::None => {}
            Step::Write { .. } | Step::Read { .. } => self.send_step(step),
            Step::Autosave => self.set_autosave(cx),
            Step::StopAutosave => {
                if let Some(editing) = self.editing() {
                    editing.autosave = editing.autosave.wrapping_add(1);
                }
            }
        }
        cx.notify();
    }

    /// Send a write or a read the model asked for (no frame is needed).
    fn send_step(&mut self, step: Step) {
        let panel = &self.areas.workspace.code;
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        let Some(path) = panel
            .files
            .viewer
            .as_ref()
            .map(|viewer| viewer.path.clone())
        else {
            return;
        };
        let ask = match step {
            Step::Write { text, base, .. } => FileAsk::Save(Save {
                root,
                path,
                text,
                base,
            }),
            Step::Read { .. } => FileAsk::Read(whole_read(root, path)),
            Step::None | Step::Autosave | Step::StopAutosave => return,
        };
        self.ask(&machine, Ask::File(ask));
    }

    /// A modified buffer written before its file is left (guarded; its
    /// answer finds no editor and is dropped). Paseo's
    /// `dispose({ flush: true })`.
    pub(super) fn flush_file(&mut self) {
        let Some(editing) = self.editing() else {
            return;
        };
        let step = editing.model.flush();
        self.send_step(step);
    }

    fn set_autosave(&mut self, cx: &mut Context<'_, Self>) {
        let Some(editing) = self.editing() else {
            return;
        };
        editing.autosave = editing.autosave.wrapping_add(1);
        let wait = editing.autosave;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(AUTOSAVE_MS))
                .await;
            let _gone = this.update(cx, |shell, cx| {
                let Some(editing) = shell.editing().filter(|editing| editing.autosave == wait)
                else {
                    return;
                };
                let step = editing.model.save();
                shell.run_step(step, cx);
            });
        })
        .detach();
    }

    /// The person typed in the editor.
    pub(super) fn editor_changed(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(editing) = self.editing() else {
            return;
        };
        let Some(text) = editing.editor.as_ref().map(|editor| editor.text(cx)) else {
            return;
        };
        let step = editing.model.edit(&text);
        self.run_step(step, cx);
    }

    /// A read of the file being edited came back. `false` when the file
    /// cannot be edited (binary, or larger than a page): the caller shows
    /// it read-only.
    pub(in crate::areas::workspace) fn editing_read(
        &mut self,
        result: &Result<FileRead, Problem>,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let words = self.words();
        let Some(editing) = self.editing() else {
            return false;
        };
        let first = editing.editor.is_none();
        let read = match result {
            Ok(read) => match &read.content {
                FileContent::Text { text, next: None }
                    if read.version.size <= MAX_EDITABLE_BYTES =>
                {
                    Read::Text(text.clone(), read.version.clone())
                }
                FileContent::Text { .. } | FileContent::Binary | FileContent::Unknown => {
                    if first {
                        return false;
                    }
                    Read::Failed(words.labels().read_failed.to_owned())
                }
            },
            Err(problem) if problem.code() == Some(ErrorCode::NotFound) => Read::Deleted,
            Err(problem) => Read::Failed(words.problem(problem)),
        };
        let before = editing.model.content().to_owned();
        let sequence = editing.model.reading();
        let step = editing.model.read(sequence, read);
        editing.stale = editing.editor.is_some() && editing.model.content() != before;
        self.run_step(step, cx);
        true
    }

    /// The node answered a save of `path`.
    pub(in crate::areas::workspace) fn editor_saved(
        &mut self,
        root: &WorkspaceRoot,
        path: &RelPath,
        saved: Saved,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.words();
        let panel = &self.areas.workspace.code;
        let ours = panel.root().is_some_and(|(_, at)| at == root)
            && panel
                .files
                .viewer
                .as_ref()
                .is_some_and(|viewer| &viewer.path == path);
        if !ours {
            return;
        }
        let Some(editing) = self.editing() else {
            return;
        };
        let Some(sequence) = editing.model.writing() else {
            return;
        };
        let written = match saved {
            Saved::Saved(version) => Written::Saved(version),
            Saved::Conflict { disk: Some(_), .. } => Written::Changed,
            Saved::Conflict { disk: None, .. } => Written::Deleted,
            Saved::Failed(failure) => Written::Failed(words.save_failure(&failure)),
        };
        let step = editing.model.saved(sequence, written);
        self.run_step(step, cx);
    }

    /// Overwrite the file that changed on disk with the buffer.
    pub(super) fn editor_overwrite(&mut self, cx: &mut Context<'_, Self>) {
        let Some(editing) = self.editing() else {
            return;
        };
        let step = editing.model.overwrite();
        self.run_step(step, cx);
    }

    /// Reload: at once for a clean buffer, else after Paseo's question.
    pub(super) fn editor_reload(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(editing) = self.editing() else {
            return;
        };
        if editing.model.modified() {
            let dialog = editing.reload.get_or_insert_with(|| DialogState::new(cx));
            dialog.focus(window, cx);
            cx.notify();
            return;
        }
        let step = editing.model.reload();
        self.run_step(step, cx);
    }

    /// Reload's question answered: take the disk's file, or keep editing.
    pub(super) fn editor_reload_answered(&mut self, reload: bool, cx: &mut Context<'_, Self>) {
        let Some(editing) = self.editing() else {
            return;
        };
        editing.reload = None;
        let step = if reload {
            editing.model.reload()
        } else {
            Step::None
        };
        self.run_step(step, cx);
    }

    /// The file is left (Back, another file): flushed, then closed.
    pub(super) fn leave_file(&mut self, cx: &mut Context<'_, Self>) {
        self.flush_file();
        self.areas.workspace.code.files.viewer = None;
        cx.notify();
    }

    /// Make the editor control after the first read, or put a reload's
    /// text in it (both need the window, which an answer does not carry).
    pub(super) fn editor_frame(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(viewer) = self.areas.workspace.code.files.viewer.as_mut() else {
            return;
        };
        let language = language_of(viewer.path.as_str());
        let wrap = wraps(viewer.path.as_str());
        let Some(editing) = viewer.edit.as_mut() else {
            return;
        };
        if editing.model.version().is_none() {
            return;
        }
        if let Some(editor) = editing.editor.as_ref() {
            if editing.stale {
                editing.stale = false;
                editor.set_text(editing.model.content(), window, cx);
            }
            return;
        }
        let editor = CodeEditor::new(
            editing.model.content(),
            ViewSyntax(language),
            Self::editor_changed,
            window,
            cx,
        );
        if wrap {
            editor.set_wrapping(true, window, cx);
        }
        let goto = viewer.goto.take();
        if let Some(line) = goto {
            editor.reveal_line(line, window, cx);
        }
        let status = super::editor_status::Status::new(editor.clone(), cx);
        let Some(editing) = viewer.edit.as_mut() else {
            return;
        };
        editing.editor = Some(editor);
        editing.status = Some(status);
        cx.notify();
    }
}
