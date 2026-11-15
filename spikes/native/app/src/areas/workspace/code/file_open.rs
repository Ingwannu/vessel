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
    /// Session and metadata lifetime are separate from `NodeId`/root/path buffer
    /// identity. A refreshed receipt cannot reinterpret an old activation.
    pub(super) fn file_open_context_is_current(&self, origin: &FileOpenContext) -> bool {
        self.file_open_context().as_ref() == Some(origin)
    }
}

/// Opening refusals never turn a different active root into a replacement intent.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOpenRefusal {
    /// Original receipt is no longer installed.
    Changed,
    /// The workspace directory itself is not a file target.
    Root,
    /// An original path mutation still owns the resource.
    Busy,
    /// Bounded retained owners cannot make space without losing work.
    Capacity,
    /// The sole layout owner refused placement.
    Layout(vessel_app_view::layout::LayoutError),
}
impl Shell {
    /// Concrete compatibility opener, staged until its real UI and close-owner
    /// dispatch join. It uses only the actual returned tab id, never a local allocator.
    #[cfg(test)]
    pub(crate) fn open_workspace_file(
        &mut self,
        origin: &FileOpenContext,
        path: vessel_proto::workspace::RelPath,
        location: Option<vessel_app_view::workspace::file_location::FileLocation>,
        placement: vessel_app_view::layout::Placement,
        window: &mut vessel_app_kit::ui::Window,
        cx: &mut vessel_app_kit::ui::Context<'_, Self>,
    ) -> Result<vessel_app_view::layout::TabId, FileOpenRefusal> {
        use super::buffers::FileKey;
        use vessel_app_view::layout::{HostAlias, LayoutKey, OpenIntent, Target};
        if !self.file_open_context_is_current(origin) {
            return Err(FileOpenRefusal::Changed);
        }
        if path.is_root() {
            return Err(FileOpenRefusal::Root);
        }
        let key = FileKey::new(origin.scope(), path.clone());
        let workspace = LayoutKey::Session {
            machine: HostAlias::parse(origin.scope().connection().machine().as_str())
                .map_err(FileOpenRefusal::Layout)?,
            session: origin.session,
        };
        let panel = &self.areas.workspace.code;
        if panel.file_mutating(&key) {
            return Err(FileOpenRefusal::Busy);
        }
        if !panel.file_tabs.can_bind(&workspace, &key) {
            return Err(FileOpenRefusal::Capacity);
        }
        let previous = panel.file_key();
        let reuse = previous.as_ref() == Some(&key)
            && panel
                .files
                .viewer
                .as_ref()
                .is_some_and(|viewer| viewer.scope == *origin.scope());
        if !reuse && !self.park_file() {
            return Err(FileOpenRefusal::Capacity);
        }
        let opened = self.layout_open_target(
            &workspace,
            Target::File {
                root: key.root,
                path: path.clone(),
            },
            OpenIntent::Reveal,
            placement,
            window,
            cx,
        );
        let tab = match opened {
            Ok(tab) => tab,
            Err(error) => {
                if !reuse && let Some(previous) = previous {
                    let files = &mut self.areas.workspace.code.files;
                    files.viewer = files.retained.remove(&previous);
                }
                return Err(FileOpenRefusal::Layout(error));
            }
        };
        self.areas.workspace.code.file_tabs.bind(
            workspace.clone(),
            tab,
            origin.clone(),
            key.clone(),
        );
        // Protect a newly allocated tab before acquisition can enqueue a reply.
        self.publish_file_tabs(&key, cx);
        if reuse {
            if let Some(viewer) = self.areas.workspace.code.viewer_for(&key)
                && location.is_some()
            {
                viewer.goto = location;
                if let Some(preview) = &mut viewer.markdown {
                    preview.reveal_source();
                }
            }
        } else {
            self.show_file(
                origin.scope().connection().machine().as_str(),
                origin.scope().root(),
                path,
                location,
            );
        }
        self.publish_file_tabs(&key, cx);
        Ok(tab)
    }
}
