//! Original write obligations, not text dirtiness, control document disposal.
//! Actual readiness reads the same engine; context-free caches retain editors.
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
    /// Read-only admission uses the actual engine before any queued mirror.
    pub(super) fn document_readiness_in(&self, cx: &gpui::App) -> DocumentReadiness {
        let known = self.document_readiness();
        if known != DocumentReadiness::Ready {
            return known;
        }
        let Some(editing) = &self.edit else {
            return known;
        };
        let Some(editor) = &editing.editor else {
            return known;
        };
        let text = editor.text(cx);
        let old_projection = editing.stale && editing.mirrored.as_deref() == Some(text.as_str());
        if !old_projection && text != editing.model.content() {
            DocumentReadiness::Dirty
        } else {
            known
        }
    }
    /// Valid only after a synchronous mirror, or without an engine at all.
    pub(super) fn can_release_snapshot(&self) -> bool {
        self.reading.is_none()
            && self
                .edit
                .as_ref()
                .is_none_or(|editing| editing.reading.is_none())
            && self.document_readiness() == DocumentReadiness::Ready
    }
    /// Without App access no cache may assume an editor's queued input is clean.
    pub(super) fn can_evict(&self) -> bool {
        self.edit
            .as_ref()
            .is_none_or(|editing| editing.editor.is_none())
            && self.can_release_snapshot()
    }
}

// The desktop owner supplies the checked native control packet and actual
// callers. Remove this staging attribute only with their live registration.
#[cfg(test)]
pub(super) fn check_documents(shell: &crate::shell::Shell, cx: &gpui::App) -> DocumentReadiness {
    check_matching(shell, None, cx)
}

impl crate::shell::Shell {
    /// Readiness facts for every original file effect on this actual node.
    /// This is not controller ownership or permission to retire a connection.
    /// Unrelated nodes and mutable route aliases do not participate.
    // Staged until the desktop owner adds its actual Stop/Restart caller.
    #[cfg(test)]
    pub(crate) fn file_documents_for_node(
        &self,
        node: vessel_proto::NodeId,
        cx: &gpui::App,
    ) -> DocumentReadiness {
        check_matching(self, Some(node), cx)
    }
}

#[cfg(test)]
fn check_matching(
    shell: &crate::shell::Shell,
    node: Option<vessel_proto::NodeId>,
    cx: &gpui::App,
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
        match (state, viewer.document_readiness_in(cx)) {
            (Failed, _) | (_, Failed) => Failed,
            (Pending, _) | (_, Pending) => Pending,
            (Dirty, _) | (_, Dirty) => Dirty,
            (Ready, Ready) => Ready,
        }
    })
}
#[cfg(test)]
mod tests;
