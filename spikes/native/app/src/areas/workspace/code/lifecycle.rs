//! Original write obligations, not text dirtiness, control document disposal.
//! One owner predicate is reused by cache eviction and path mutation admission.
use super::Viewer;
use vessel_app_view::desktop::quit::DocumentReadiness;
use vessel_app_view::workspace::editor::{EditorStatus, WriteState};

impl Viewer {
    pub(super) fn document_readiness(&self) -> DocumentReadiness {
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

// The desktop owner supplies the checked native control packet and actual
// callers. Remove this staging attribute only with their live registration.
#[cfg(test)]
pub(super) fn check_documents(shell: &crate::shell::Shell) -> DocumentReadiness {
    check_matching(shell, None)
}

impl crate::shell::Shell {
    /// Readiness facts for every original file effect on this actual node.
    /// This is not controller ownership or permission to retire a connection.
    /// Unrelated nodes and mutable route aliases do not participate.
    // Staged until the desktop owner adds its actual Stop/Restart caller.
    #[cfg(test)]
    pub(crate) fn file_documents_for_node(&self, node: vessel_proto::NodeId) -> DocumentReadiness {
        check_matching(self, Some(node))
    }
}

#[cfg(test)]
fn check_matching(
    shell: &crate::shell::Shell,
    node: Option<vessel_proto::NodeId>,
) -> DocumentReadiness {
    let panel = &shell.areas.workspace.code;
    let includes = |actual| node.is_none_or(|wanted| wanted == actual);
    let initial = panel
        .mutation
        .as_ref()
        .filter(|pending| includes(pending.scope.node()))
        .map_or(DocumentReadiness::Ready, |pending| {
            if pending.uncertain {
                DocumentReadiness::Failed
            } else {
                DocumentReadiness::Pending
            }
        });
    let active = panel
        .files
        .viewer
        .iter()
        .filter(|viewer| includes(viewer.scope.node()));
    let parked = panel
        .files
        .retained
        .iter()
        .filter_map(|(key, viewer)| includes(key.node).then_some(viewer));
    active.chain(parked).fold(initial, |state, viewer| {
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
