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
        self.file_open_context().is_some_and(|current| {
            current.scope == origin.scope
                && current.session == origin.session
                && current.receipt == origin.receipt
        })
    }
}

/// Opening refusals never turn a different active root into a replacement intent.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOpenRefusal {
    /// Original receipt is no longer installed.
    Changed,
    /// A reused compatibility key still belongs to another actual node.
    DifferentNode,
    /// The workspace directory itself is not a file target.
    Root,
    /// An original path mutation still owns the resource.
    Busy,
    /// Bounded retained owners cannot make space without losing work.
    Capacity,
    /// The sole layout owner refused placement.
    Layout(vessel_app_view::layout::LayoutError),
}
// Preparing placement moves only local ownership. No file request is sent
// before the sole layout owner accepts the actual target/placement.
#[cfg(test)]
struct PreparedOpen {
    workspace: vessel_app_view::layout::LayoutKey,
    file: super::buffers::FileKey,
    active: ActiveFile,
}
#[cfg(test)]
enum ActiveFile {
    Reused,
    Parked(Option<super::buffers::FileKey>),
}
impl Shell {
    #[cfg(test)]
    fn prepare_file_open(
        &mut self,
        origin: &FileOpenContext,
        path: vessel_proto::workspace::RelPath,
    ) -> Result<PreparedOpen, FileOpenRefusal> {
        use super::buffers::FileKey;
        use vessel_app_view::layout::{HostAlias, LayoutKey};
        if !self.file_open_context_is_current(origin) {
            return Err(FileOpenRefusal::Changed);
        }
        if path.is_root() {
            return Err(FileOpenRefusal::Root);
        }
        let file = FileKey::new(origin.scope(), path);
        let workspace = LayoutKey::Session {
            machine: HostAlias::parse(origin.scope().connection().machine().as_str())
                .map_err(FileOpenRefusal::Layout)?,
            session: origin.session,
        };
        let panel = &self.areas.workspace.code;
        if panel.file_tabs.conflicts(&workspace, &file) {
            return Err(FileOpenRefusal::DifferentNode);
        }
        if panel.file_mutating(&file) {
            return Err(FileOpenRefusal::Busy);
        }
        if !panel.file_tabs.can_bind(&workspace, &file) {
            return Err(FileOpenRefusal::Capacity);
        }
        let previous = panel.file_key();
        let reuse = previous.as_ref() == Some(&file)
            && panel
                .files
                .viewer
                .as_ref()
                .is_some_and(|viewer| viewer.scope == *origin.scope());
        let active = if reuse {
            ActiveFile::Reused
        } else {
            if !self.park_file() {
                return Err(FileOpenRefusal::Capacity);
            }
            ActiveFile::Parked(previous)
        };
        Ok(PreparedOpen {
            workspace,
            file,
            active,
        })
    }
    #[cfg(test)]
    fn install_file_open(
        &mut self,
        origin: &FileOpenContext,
        prepared: PreparedOpen,
        tab: vessel_app_view::layout::TabId,
        location: Option<vessel_app_view::workspace::file_location::FileLocation>,
        cx: &mut vessel_app_kit::ui::Context<'_, Self>,
    ) {
        let PreparedOpen {
            workspace,
            file,
            active,
        } = prepared;
        self.areas
            .workspace
            .code
            .file_tabs
            .bind(workspace, tab, origin.clone(), file.clone());
        // Protect a newly allocated tab before acquisition can enqueue a reply.
        self.publish_file_tabs(&file, cx);
        match active {
            ActiveFile::Reused => {
                if let Some(viewer) = self.areas.workspace.code.viewer_for(&file)
                    && location.is_some()
                {
                    viewer.goto = location;
                    if let Some(preview) = &mut viewer.markdown {
                        preview.reveal_source();
                    }
                }
            }
            ActiveFile::Parked(_) => self.show_file(
                origin.scope().connection().machine().as_str(),
                origin.scope().root(),
                file.path.clone(),
                location,
            ),
        }
        self.publish_file_tabs(&file, cx);
    }
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
        use vessel_app_view::layout::{OpenIntent, Target};
        let prepared = self.prepare_file_open(origin, path)?;
        let opened = self.layout_open_target(
            &prepared.workspace,
            Target::File {
                root: prepared.file.root,
                path: prepared.file.path.clone(),
            },
            OpenIntent::Reveal,
            placement,
            window,
            cx,
        );
        match opened {
            Ok(tab) => {
                self.install_file_open(origin, prepared, tab, location, cx);
                Ok(tab)
            }
            Err(error) => {
                if let ActiveFile::Parked(Some(previous)) = prepared.active {
                    let files = &mut self.areas.workspace.code.files;
                    files.viewer = files.retained.remove(&previous);
                }
                Err(FileOpenRefusal::Layout(error))
            }
        }
    }
}
