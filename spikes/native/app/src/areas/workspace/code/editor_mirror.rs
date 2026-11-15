//! The engine's last published text distinguishes new input from a pending
//! model-to-control projection. No second editor, edit queue or lease exists here.
use super::buffers::FileKey;
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::workspace::editor::Step;

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
        let Some(text) = editing.editor.as_ref().map(|editor| editor.text(cx)) else {
            return;
        };
        if editing.stale && editing.mirrored.as_deref() == Some(text.as_str()) {
            // This is unchanged old display text, not accepted new input.
            return;
        }
        let unprojected = editing.stale;
        editing.stale = false;
        if editing.model.content() == text {
            editing.mirrored = Some(text);
            return;
        }
        let step = if unprojected {
            editing.model.edit_before_projection(&text)
        } else {
            editing.model.edit(&text)
        };
        // Reuse this already allocated snapshot; it is comparison evidence only,
        // never a writable buffer, undo history, connection or version authority.
        editing.mirrored = Some(text);
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
