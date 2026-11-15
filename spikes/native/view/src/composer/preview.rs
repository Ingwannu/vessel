//! A pending message owns its pre-upload presentation, without remote references.
//! The timeline ledger retains this value through uncertainty and drops it only
//! on canonical reconciliation; no second send/preview registry is introduced.
use super::{
    context::{ContextAttachment, ContextTray},
    tray::{Origin, Tray},
};
use vessel_proto::{AttachmentKind, AttachmentName, conversation::MimeType};

/// File metadata and the original byte lease, independent of upload progress.
#[derive(Clone, PartialEq, Eq)]
pub struct File {
    /// Validated display name; it is never used as a managed path.
    pub name: AttachmentName,
    /// Validated wire media type.
    pub mime: MimeType,
    /// Existing node attachment classification.
    pub kind: AttachmentKind,
    /// Original admitted length, with no uploaded-byte counter.
    pub size: u64,
    /// Cloning a managed source retains its Arc lease until reconciliation.
    pub source: Origin,
}
impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("kind", &self.kind)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}
/// Immutable presentation captured once before preparation mutates the tray.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    files: Vec<File>,
    contexts: Vec<ContextAttachment>,
}
impl std::fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Snapshot")
            .field("files", &self.files.len())
            .field("contexts", &self.contexts.len())
            .finish()
    }
}
impl Snapshot {
    /// Capture bounded existing trays without I/O, upload ids or fabricated refs.
    #[must_use]
    pub fn capture(files: Option<&Tray>, context: Option<&ContextTray>) -> Self {
        Self {
            files: files
                .into_iter()
                .flat_map(Tray::items)
                .map(|item| File {
                    name: item.name.clone(),
                    mime: item.mime.clone(),
                    kind: item.kind,
                    size: item.size,
                    source: item.origin.clone(),
                })
                .collect(),
            contexts: context
                .into_iter()
                .flat_map(ContextTray::visible)
                .cloned()
                .collect(),
        }
    }
    /// Borrow the retained original file list; drawing never clones its bytes.
    #[must_use]
    pub fn files(&self) -> &[File] {
        &self.files
    }
    /// Borrow only context which was visible at admission, in its original order.
    #[must_use]
    pub fn contexts(&self) -> &[ContextAttachment] {
        &self.contexts
    }
    /// Empty compatibility admission needs no special alternate model.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.contexts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn pending_snapshot_keeps_original_bytes_after_tray_mutation_and_debug_redacts() {
        let bytes: Arc<[u8]> = Arc::from([1, 2, 3]);
        let mut tray = Tray::default();
        let key = tray
            .add("private-name.png", 3, Origin::Pasted(Arc::clone(&bytes)))
            .unwrap();
        let snapshot = Snapshot::capture(Some(&tray), None);
        let _removed = tray.remove(key);
        assert_eq!(snapshot.files().len(), 1);
        assert!(
            matches!(&snapshot.files()[0].source,Origin::Pasted(held) if Arc::ptr_eq(held,&bytes))
        );
        assert!(!format!("{snapshot:?}").contains("private-name"));
        assert!(!format!("{:?}", snapshot.files()).contains("private-name"));
        assert!(!snapshot.is_empty());
        assert!(Snapshot::default().is_empty());
    }
}
