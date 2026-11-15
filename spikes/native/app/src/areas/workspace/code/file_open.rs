//! An opening retains the installed root receipt, not just reusable buffer identity.
//! These are correlation facts; the shared actor still admits every file effect.
use crate::shell::Shell;
use vessel_app_view::workspace::{file_scope::FileScope, file_ticket::FileTicket};
use vessel_proto::SessionId;

/// Original session placement and physical file scope captured before queued UI.
/// Only the file owner constructs it from its installed metadata receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileOpenContext {
    scope: FileScope,
    session: SessionId,
    receipt: FileTicket,
}
impl FileOpenContext {
    /// Search/read facts stay bound to the original connection and physical root.
    /// Borrowing them does not grant filesystem or layout authority.
    pub(crate) const fn scope(&self) -> &FileScope {
        &self.scope
    }
}
impl Shell {
    /// Capture the exact active session/root receipt, without resolving a new
    /// workspace, allocating a tab or finding a node by mutable alias.
    pub(crate) fn file_open_context(&self) -> Option<FileOpenContext> {
        let scope = self.active_file_scope()?;
        let target = self.areas.workspace.code.target.as_ref()?;
        let admission = target.admission.as_ref()?;
        Some(FileOpenContext {
            scope,
            session: target.session,
            receipt: admission.ticket,
        })
    }
    /// Session and metadata lifetime are separate from NodeId/root/path buffer
    /// identity. A refreshed receipt cannot reinterpret an old activation.
    pub(super) fn file_open_context_is_current(&self, origin: &FileOpenContext) -> bool {
        self.file_open_context().as_ref() == Some(origin)
    }
}
