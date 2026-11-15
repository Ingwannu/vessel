//! One file-owner decision over the layout's immutable close snapshot.
//! This holds new effects, never cancels or discharges an original remote write.
use super::{FileKey, FileOpenContext};
use crate::shell::Shell;
#[cfg(test)]
use vessel_app_kit::ui::Window;
use vessel_app_kit::{dialog::DialogState, ui::Context};
#[cfg(test)]
use vessel_app_view::layout::Target;
use vessel_app_view::{
    layout::{ClosePlan, LayoutError, LayoutKey, Modified},
    workspace::{
        editor::{EditorStatus, Step},
        file_ticket::FileTicket,
    },
};

pub(super) struct ClosePrompt {
    pub(super) workspace: LayoutKey,
    pub(super) plan: ClosePlan,
    pub(super) file: FileKey,
    origin: FileOpenContext,
    draft: String,
    pub(super) ticket: FileTicket,
    pub(super) dialog: DialogState,
}

/// Only a non-file target may continue to another owner.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileCloseRequest {
    NotOwned,
    Handled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Decision {
    Keep,
    Discard,
}

fn unsettled(viewer: &super::super::Viewer) -> bool {
    viewer.reading.is_some()
        || viewer.edit.as_ref().is_some_and(|editing| {
            editing.reading.is_some()
                || editing.reload.is_some()
                || editing.writing.is_some()
                || editing.model.writing().is_some()
                || editing.model.write_state().is_some()
        })
}

impl Shell {
    /// Staged until the sole layout dispatcher invokes this before ANY file close,
    /// including a tab whose cached modified bit still says Clean.
    #[cfg(test)]
    pub(crate) fn request_layout_file_close(
        &mut self,
        workspace: &LayoutKey,
        plan: &ClosePlan,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> FileCloseRequest {
        let Target::File { root, path } = plan.target() else {
            return FileCloseRequest::NotOwned;
        };
        let binding = self
            .areas
            .workspace
            .code
            .file_tabs
            .bindings
            .get(&(workspace.clone(), plan.tab()));
        let Some((file, origin)) = binding
            .filter(|binding| binding.file.root == *root && binding.file.path == *path)
            .map(|binding| (binding.file.clone(), binding.origin.clone()))
        else {
            self.layout_close_settled(workspace, plan, Err(LayoutError::InvalidState), cx);
            return FileCloseRequest::Handled;
        };
        self.mirror_editor_for(&file, cx);
        let panel = &self.areas.workspace.code;
        if !self.layout_tab_matches(workspace, plan.tab(), plan.target())
            || panel.file_effect_held(&file)
            || panel.file_tabs.close.is_some()
            || panel.viewer_at(&file).is_none_or(unsettled)
        {
            self.layout_close_settled(workspace, plan, Err(LayoutError::Modified), cx);
            return FileCloseRequest::Handled;
        }
        let viewer = panel.viewer_at(&file);
        let draft = viewer
            .and_then(|viewer| viewer.edit.as_ref())
            .filter(|editing| editing.model.modified())
            .map(|editing| editing.model.content().to_owned());
        if let Some(draft) = draft {
            let Some(ticket) = self.next_file_ticket() else {
                return FileCloseRequest::Handled;
            };
            self.run_step_for(file.clone(), Step::StopAutosave, cx);
            let dialog = DialogState::new(cx);
            dialog.focus(window, cx);
            self.areas.workspace.code.file_tabs.close = Some(ClosePrompt {
                workspace: workspace.clone(),
                plan: plan.clone(),
                file: file.clone(),
                origin,
                draft,
                ticket,
                dialog,
            });
            self.set_file_close_editability(&file, cx);
            cx.notify();
        } else {
            self.finish_file_presentation(workspace, plan, &file, cx);
        }
        FileCloseRequest::Handled
    }

    fn set_file_close_editability(&self, file: &FileKey, cx: &mut Context<'_, Self>) {
        let panel = &self.areas.workspace.code;
        let policy = if panel.file_effect_held(file) {
            vessel_app_kit::Editability::ReadOnly
        } else {
            vessel_app_kit::Editability::Editable
        };
        if let Some(editor) = panel
            .viewer_at(file)
            .and_then(|viewer| viewer.edit.as_ref())
            .and_then(|editing| editing.editor.as_ref())
        {
            editor.set_editability(policy, cx);
        }
    }

    pub(super) fn file_close_answered(
        &mut self,
        ticket: FileTicket,
        decision: Decision,
        cx: &mut Context<'_, Self>,
    ) {
        let file = self
            .areas
            .workspace
            .code
            .file_tabs
            .close
            .as_ref()
            .filter(|prompt| prompt.ticket == ticket)
            .map(|prompt| prompt.file.clone());
        let Some(file) = file else {
            return;
        };
        // Preserve any accepted text whose mirror was queued before the readonly hold.
        self.mirror_editor_for(&file, cx);
        let Some(prompt) = self.areas.workspace.code.file_tabs.close.take() else {
            return;
        };
        self.set_file_close_editability(&file, cx);
        if decision == Decision::Keep {
            if self.editing_for(&file).is_some_and(|editing| {
                editing.model.status() == EditorStatus::Dirty
                    && editing.model.write_state().is_none()
            }) {
                self.run_step_for(file, Step::Autosave, cx);
            }
            cx.notify();
            return;
        }
        let panel = &self.areas.workspace.code;
        let original = panel
            .file_tabs
            .bindings
            .get(&(prompt.workspace.clone(), prompt.plan.tab()))
            .is_some_and(|binding| binding.file == file && binding.origin == prompt.origin);
        let safe = original
            && !panel.file_effect_held(&file)
            && panel.viewer_at(&file).is_some_and(|viewer| {
                !unsettled(viewer)
                    && viewer
                        .edit
                        .as_ref()
                        .is_some_and(|editing| editing.model.content() == prompt.draft)
            });
        if safe {
            self.finish_file_presentation(&prompt.workspace, &prompt.plan, &file, cx);
        } else {
            self.layout_close_settled(
                &prompt.workspace,
                &prompt.plan,
                Err(LayoutError::Modified),
                cx,
            );
        }
    }

    fn finish_file_presentation(
        &mut self,
        workspace: &LayoutKey,
        plan: &ClosePlan,
        file: &FileKey,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.layout_tab_matches(workspace, plan.tab(), plan.target()) {
            return;
        }
        // Explicit discard approves only this presentation. Other bindings retain
        // their original draft and dirty status; no global model is made Clean.
        let ready = self.update_layout(
            workspace,
            |layout| layout.set_modified(plan.tab(), Modified::Clean),
            cx,
        );
        if ready.is_err() {
            return;
        }
        self.layout_close_settled(workspace, plan, Ok(()), cx);
        if self.layout_tab_exists(workspace, plan.tab()) {
            return;
        }
        let panel = &mut self.areas.workspace.code;
        let _closed = panel
            .file_tabs
            .bindings
            .remove(&(workspace.clone(), plan.tab()));
        if !panel.file_tabs.contains(file) {
            if panel.file_key().as_ref() == Some(file) {
                panel.files.viewer = None;
            }
            let _draft = panel.files.retained.remove(file);
        }
        cx.notify();
    }
}
