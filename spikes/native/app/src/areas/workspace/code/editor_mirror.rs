//! The engine's last published text distinguishes new input from a pending
//! model-to-control projection. No second editor, edit queue or lease exists here.
use super::buffers::FileKey;
use crate::shell::Shell;
use vessel_app_kit::code_editor::{CodeEditor, EditorRevision};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::workspace::editor::Step;

/// One published control observation: equal text can still contain new edits.
/// The revision belongs only to this retained engine, never to a disk write.
#[derive(PartialEq, Eq)]
pub(super) struct Snapshot {
    text: String,
    revision: EditorRevision,
}
impl Snapshot {
    pub(super) fn read(editor: &CodeEditor, cx: &gpui::App) -> Self {
        Self {
            text: editor.text(cx),
            revision: editor.revision(cx),
        }
    }
    pub(super) fn text(&self) -> &str {
        &self.text
    }
}

impl Shell {
    pub(super) fn editor_changed_for(
        &mut self,
        key: &FileKey,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.mirror_editor_for(key, cx);
    }

    /// Admission and projection must inspect the actual existing engine before
    /// replacing/dropping it, even if its change notification is still queued.
    pub(super) fn mirror_editor_for(&mut self, key: &FileKey, cx: &mut Context<'_, Self>) {
        let held = self.areas.workspace.code.file_effect_held(key);
        let Some(editing) = self.editing_for(key) else {
            return;
        };
        let Some(snapshot) = editing
            .editor
            .as_ref()
            .map(|editor| Snapshot::read(editor, cx))
        else {
            return;
        };
        if editing.stale && editing.mirrored.as_ref() == Some(&snapshot) {
            // This is unchanged old display text, not accepted new input.
            return;
        }
        let unprojected = editing.stale;
        editing.stale = false;
        if editing.model.content() == snapshot.text {
            editing.mirrored = Some(snapshot);
            return;
        }
        let step = if unprojected {
            editing.model.edit_before_projection(&snapshot.text)
        } else {
            editing.model.edit(&snapshot.text)
        };
        // Reuse this already allocated snapshot; it is comparison evidence only,
        // never a writable buffer, undo history, connection or version authority.
        editing.mirrored = Some(snapshot);
        let step = if held && matches!(&step, Step::Autosave) {
            Step::StopAutosave
        } else {
            step
        };
        if let Some(viewer) = self.areas.workspace.code.viewer_for(key) {
            viewer.intelligence = super::intelligence::State::Idle;
            if let Some(preview) = &mut viewer.markdown {
                preview.invalidate();
            }
        }
        self.run_step_for(key, step, cx);
    }
}
