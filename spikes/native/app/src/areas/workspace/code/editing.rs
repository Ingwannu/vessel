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

use vessel_app_kit::code_editor::CodeEditor;
use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::workspace::editor::{
    EditorModel, MAX_EDITABLE_BYTES, Read, Step, Written, wraps,
};
use vessel_app_view::workspace::file_ticket::FileTicket;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::save::Saved;
use vessel_app_view::workspace::syntax::language_of;
use vessel_proto::ErrorCode;
use vessel_proto::workspace::{FileContent, FileRead, ReadFile, RelPath, WorkspaceRoot};

use super::line_syntax::ViewSyntax;
use crate::shell::Shell;

/// A file being edited.
pub(super) struct Editing {
    pub(super) find: Option<vessel_app_kit::ui::Entity<super::editor_find::FindPanel>>,
    pub(super) model: EditorModel,
    /// Made on the frame after the first read (it needs the window).
    pub(super) editor: Option<CodeEditor>,
    /// The caret's line and column and the selection's button.
    pub(super) status: Option<vessel_app_kit::ui::Entity<super::editor_status::Status>>,
    /// The model's text is newer than the control's (a reload).
    pub(super) stale: bool,
    /// Bumped by every autosave set or stopped; a timer checks it.
    pub(super) autosave: u64,
    /// Reload's question, made when first asked.
    pub(super) reload: Option<ReloadPrompt>,
    /// Link correlation and the model sequence captured when sent.
    pub(super) reading: Option<(FileTicket, u64)>,
    pub(super) writing: Option<WriteTicket>,
}

/// Original attempted write survives lease changes and ambiguous completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WriteTicket {
    pub(super) ticket: FileTicket,
    pub(super) sequence: u64,
    pub(super) scope: vessel_app_view::workspace::file_scope::FileScope,
}

/// An answer must belong to the exact opening, even at the same file path.
pub(super) struct ReloadPrompt {
    pub(super) ticket: FileTicket,
    pub(super) dialog: DialogState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReloadDecision {
    Keep,
    Reload,
}

impl Editing {
    pub(super) const fn new() -> Self {
        Self {
            find: None,
            model: EditorModel::new(),
            editor: None,
            status: None,
            stale: false,
            autosave: 0,
            reload: None,
            reading: None,
            writing: None,
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
    pub(super) fn editor_save_for(
        &mut self,
        key: &super::buffers::FileKey,
        cx: &mut Context<'_, Self>,
    ) {
        if self.refuse_file_mutation(key) {
            cx.notify();
            return;
        }
        let Some(editing) = self.editing_for(key) else {
            return;
        };
        let step = editing.model.save();
        self.run_step_for(key.clone(), step, cx);
    }

    /// The person typed in the editor.
    pub(super) fn editor_changed_for(
        &mut self,
        key: &super::buffers::FileKey,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.mirror_editor_for(key, cx);
    }

    // Already accepted engine text must reach the owner even when a path hold
    // has begun; a hold rejects new effects, never publication of prior input.
    pub(super) fn mirror_editor_for(
        &mut self,
        key: &super::buffers::FileKey,
        cx: &mut Context<'_, Self>,
    ) {
        let held = self.areas.workspace.code.file_mutating(key);
        let Some(editing) = self.editing_for(key) else {
            return;
        };
        let Some(text) = editing.editor.as_ref().map(|editor| editor.text(cx)) else {
            return;
        };
        if editing.stale || editing.model.content() == text {
            return;
        }
        let step = match editing.model.edit(&text) {
            Step::Autosave if held => Step::StopAutosave,
            step => step,
        };
        if let Some(viewer) = self.areas.workspace.code.viewer_for(key) {
            viewer.intelligence = super::intelligence::State::Idle;
            if let Some(preview) = &mut viewer.markdown {
                preview.invalidate();
            }
        }
        self.run_step_for(key.clone(), step, cx);
    }

    /// A read of the file being edited came back. `false` when the file
    /// cannot be edited (binary, or larger than a page): the caller shows
    /// it read-only.
    pub(super) fn editing_read(
        &mut self,
        key: &super::buffers::FileKey,
        ticket: FileTicket,
        result: &Result<FileRead, Problem>,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let words = self.words();
        let Some(editing) = self.editing_for(key) else {
            // No matching editor owns this response anymore; only a
            // current ticket may choose a readonly fallback below.
            return true;
        };
        let Some((_, sequence)) = editing.reading.take_if(|(at, _)| *at == ticket) else {
            return true;
        };
        let first = editing.model.version().is_none();
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
                    Read::Unverified
                }
            },
            Err(problem) if problem.code() == Some(ErrorCode::NotFound) => Read::Deleted,
            Err(problem) => Read::Failed(words.problem(problem)),
        };
        let before = editing.model.content().to_owned();
        let ended = (editing.model.write_state()
            == Some(vessel_app_view::workspace::editor::WriteState::Ended))
        .then(|| editing.model.writing())
        .flatten();
        let step = editing.model.read(sequence, read);
        if let Some(original) = ended
            && editing.model.writing().is_none()
            && editing
                .writing
                .as_ref()
                .is_some_and(|attempt| attempt.sequence == original)
        {
            editing.writing = None;
        }
        editing.stale = editing.editor.is_some() && editing.model.content() != before;
        if let Some(viewer) = self.areas.workspace.code.viewer_for(key)
            && let Some(preview) = &mut viewer.markdown
        {
            preview.invalidate();
        }
        self.run_step_for(key.clone(), step, cx);
        self.reconcile_watched_file(key, cx);
        true
    }

    /// The node answered a save of `path`.
    pub(super) fn editor_saved_for(
        &mut self,
        key: &super::buffers::FileKey,
        scope: &vessel_app_view::workspace::file_scope::FileScope,
        ticket: FileTicket,
        current: bool,
        saved: Saved,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.words();
        let Some(editing) = self.editing_for(key) else {
            return;
        };
        let Some(attempt) = editing
            .writing
            .as_ref()
            .filter(|attempt| attempt.ticket == ticket && attempt.scope == *scope)
        else {
            return;
        };
        let sequence = attempt.sequence;
        if !current {
            editing.model.retire_observations();
        }
        if !matches!(saved, Saved::Uncertain(_) | Saved::Ended(_)) {
            editing.writing = None;
        }
        let written = match saved {
            Saved::Uncertain(problem) => Written::Uncertain(words.problem(&problem)),
            Saved::Ended(problem) => Written::Ended(words.problem(&problem)),
            Saved::Saved(version) if current => Written::Saved(version),
            Saved::Saved(_) => Written::UnverifiedConflict,
            Saved::Missing { .. } if current => Written::Deleted,
            Saved::Missing { .. } => Written::UnverifiedConflict,
            Saved::Conflict { disk: Some(_), .. } if current => Written::Changed,
            Saved::Conflict { .. } => Written::UnverifiedConflict,
            Saved::Failed(failure) => Written::Failed(words.save_failure(&failure)),
        };
        let step = editing.model.saved(sequence, written);
        if editing.model.write_state()
            == Some(vessel_app_view::workspace::editor::WriteState::Ended)
        {
            // Step::Read is one effect, so retire pre-error autosave timers
            // here before sending that read; only a new user intent resumes.
            editing.autosave = editing.autosave.wrapping_add(1);
        }
        let step = if !current && matches!(&step, Step::Read { .. }) {
            Step::None
        } else {
            step
        };
        self.run_step_for(key.clone(), step, cx);
        self.reconcile_watched_file(key, cx);
    }

    /// Only a terminal original response permits a new confirmation read. The
    /// original write ticket remains held until that keyed read can reconcile it.
    pub(super) fn editor_confirm_saved_for(
        &mut self,
        key: &super::buffers::FileKey,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(editing) = self.editing_for(key) else {
            return;
        };
        if editing.model.write_state()
            != Some(vessel_app_view::workspace::editor::WriteState::Ended)
            || editing.reading.is_some()
        {
            return;
        }
        let step = editing.model.reconcile();
        self.run_step_for(key.clone(), step, cx);
    }

    /// Conflict actions keep the originating buffer when their click is queued.
    pub(super) fn editor_overwrite(
        &mut self,
        key: &super::buffers::FileKey,
        cx: &mut Context<'_, Self>,
    ) {
        if self.refuse_file_mutation(key) {
            cx.notify();
            return;
        }
        let Some(editing) = self.editing_for(key) else {
            return;
        };
        let step = editing.model.overwrite();
        self.run_step_for(key.clone(), step, cx);
    }

    pub(super) fn editor_reload(
        &mut self,
        key: &super::buffers::FileKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.refuse_file_mutation(key) {
            cx.notify();
            return;
        }
        let Some(editing) = self.editing_for(key) else {
            return;
        };
        if !editing.model.modified() {
            let step = editing.model.reload();
            self.run_step_for(key.clone(), step, cx);
            return;
        }
        let opening = editing.reload.is_none();
        let ticket = if opening {
            self.next_file_ticket()
        } else {
            None
        };
        let Some(editing) = self.editing_for(key) else {
            return;
        };
        if let Some(ticket) = ticket {
            editing.reload = Some(ReloadPrompt {
                ticket,
                dialog: DialogState::new(cx),
            });
        }
        if let Some(prompt) = &mut editing.reload {
            prompt.dialog.focus(window, cx);
        }
        cx.notify();
    }

    pub(super) fn editor_reload_answered(
        &mut self,
        key: &super::buffers::FileKey,
        ticket: FileTicket,
        choice: ReloadDecision,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(editing) = self.editing_for(key) else {
            return;
        };
        if editing
            .reload
            .as_ref()
            .is_none_or(|prompt| prompt.ticket != ticket)
        {
            return;
        }
        editing.reload = None;
        let step = match choice {
            ReloadDecision::Reload => editing.model.reload(),
            ReloadDecision::Keep => Step::None,
        };
        self.run_step_for(key.clone(), step, cx);
    }

    /// The file is left (Back, another file): flushed, then closed.
    pub(super) fn leave_file_for(
        &mut self,
        key: &super::buffers::FileKey,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.workspace.code.file_key().as_ref() != Some(key) {
            return;
        }
        self.flush_file();
        let _parked = self.park_file();
        cx.notify();
    }

    /// Make the editor control after the first read, or put a reload's
    /// text in it (both need the window, which an answer does not carry).
    #[cfg(test)]
    pub(super) fn editor_frame(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(key) = self.areas.workspace.code.file_key() {
            self.editor_frame_for(&key, window, cx);
        }
    }

    pub(super) fn editor_frame_for(
        &mut self,
        key: &super::buffers::FileKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let mutation_held = self.areas.workspace.code.file_mutating(key);
        let labels = self.words().labels();
        let Some(viewer) = self.areas.workspace.code.viewer_for(key) else {
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
            if let Some(location) = viewer.goto.take() {
                super::editor_io::reveal_location(editor, location, window, cx);
            }
            if editing.stale {
                editing.stale = false;
                editor.set_text(editing.model.content(), window, cx);
            }
            return;
        }
        let editor = CodeEditor::new(
            editing.model.content(),
            ViewSyntax(language),
            {
                let key = key.clone();
                move |shell: &mut Shell, window, cx| shell.editor_changed_for(&key, window, cx)
            },
            window,
            cx,
        );
        editor.set_editability(
            if mutation_held {
                vessel_app_kit::Editability::ReadOnly
            } else {
                vessel_app_kit::Editability::Editable
            },
            cx,
        );
        if wrap {
            editor.set_wrapping(true, window, cx);
        }
        let goto = viewer.goto.take();
        if let Some(location) = goto {
            super::editor_io::reveal_location(&editor, location, window, cx);
        }
        let status = super::editor_status::Status::new(editor.clone(), key.clone(), cx);
        let find =
            super::editor_find::FindPanel::new(editor.clone(), key.clone(), labels, window, cx);
        let Some(editing) = viewer.edit.as_mut() else {
            return;
        };
        editing.editor = Some(editor);
        editing.status = Some(status);
        editing.find = Some(find);
        cx.notify();
    }
}
