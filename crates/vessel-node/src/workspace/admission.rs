//! Retain the actual nearest registered ancestor FD, never a path-derived authority.
use super::{WorkspaceError, Workspaces};
use std::{path::Path, sync::Arc};
use vessel_workspace::physical::{Ancestry, Directory};
pub(super) struct OpenedAdmission {
    pub(super) directory: Arc<Directory>,
    pub(super) root_directory: Arc<Directory>,
    pub(super) ancestry: Arc<Ancestry>,
}
impl Workspaces {
    pub(super) async fn opened_admission(
        &self,
        path: &Path,
    ) -> Result<OpenedAdmission, WorkspaceError> {
        let path = path.to_path_buf();
        let registered = {
            let state = self.state();
            state
                .projects
                .values()
                .map(|entry| entry.info.root.clone())
                .chain(
                    state
                        .worktrees
                        .values()
                        .map(|entry| entry.info.path.clone()),
                )
                .collect::<Vec<_>>()
        };
        self.root_metadata(move || {
            let directory = Arc::new(Directory::open(&path)?);
            let ancestry = directory.ancestry()?;
            let mut root_directory: Option<Arc<Directory>> = None;
            for path in registered {
                match Directory::open(&path) {
                    Ok(opened) => {
                        let identity = opened.identity();
                        let candidates = root_directory
                            .iter()
                            .map(|root| root.identity())
                            .chain(std::iter::once(identity))
                            .collect();
                        if ancestry.nearest(&candidates) == Some(identity) {
                            root_directory = Some(Arc::new(opened));
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            // Keeping only the selected ancestor bounds descriptors independently
            // of registry size and prevents inode reuse while its lease lives.
            let root_directory = root_directory.unwrap_or_else(|| Arc::clone(&directory));
            let ancestry = Arc::new(root_directory.ancestry()?);
            Ok(OpenedAdmission {
                directory,
                root_directory,
                ancestry,
            })
        })
        .await
    }
}
