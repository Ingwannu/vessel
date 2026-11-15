//! Original write obligations, not text dirtiness, control document disposal.
//! One owner predicate is reused by cache eviction and path mutation admission.
use super::Viewer;
use vessel_app_view::desktop::quit::DocumentReadiness;
use vessel_app_view::workspace::editor::{EditorStatus, WriteState};

impl Viewer {
    fn document_readiness(&self) -> DocumentReadiness {
        let Some(editing) = &self.edit else {
            return DocumentReadiness::Ready;
        };
        if editing.model.write_state() == Some(WriteState::Uncertain) {
            return DocumentReadiness::Failed;
        }
        if editing.model.write_state() == Some(WriteState::Ended) {
            return if editing.reading.is_some() {
                DocumentReadiness::Pending
            } else {
                DocumentReadiness::Failed
            };
        }
        if editing.writing.is_some() || editing.model.writing().is_some() {
            return DocumentReadiness::Pending;
        }
        if !editing.model.modified() {
            return DocumentReadiness::Ready;
        }
        match editing.model.status() {
            EditorStatus::Failed | EditorStatus::Error | EditorStatus::Conflict => {
                DocumentReadiness::Failed
            }
            EditorStatus::Loading
            | EditorStatus::Clean
            | EditorStatus::Dirty
            | EditorStatus::Saving => DocumentReadiness::Dirty,
        }
    }
    pub(super) fn can_evict(&self) -> bool {
        self.reading.is_none()
            && self
                .edit
                .as_ref()
                .is_none_or(|editing| editing.reading.is_none())
            && self.document_readiness() == DocumentReadiness::Ready
    }
}

// The desktop owner supplies the checked native control packet and the actual
// finalizer. Remove this staging attribute only with its live registration.
#[cfg(test)]
pub(super) fn check_documents(shell: &crate::shell::Shell) -> DocumentReadiness {
    let panel = &shell.areas.workspace.code;
    let initial = panel
        .mutation
        .as_ref()
        .map_or(DocumentReadiness::Ready, |pending| {
            if pending.uncertain {
                DocumentReadiness::Failed
            } else {
                DocumentReadiness::Pending
            }
        });
    panel
        .files
        .viewer
        .iter()
        .chain(panel.files.retained.values())
        .fold(initial, |state, viewer| {
            use DocumentReadiness::{Dirty, Failed, Pending, Ready};
            match (state, viewer.document_readiness()) {
                (Failed, _) | (_, Failed) => Failed,
                (Pending, _) | (_, Pending) => Pending,
                (Dirty, _) | (_, Dirty) => Dirty,
                (Ready, Ready) => Ready,
            }
        })
}
#[cfg(test)]
mod tests;
