//! The file calls in a project's or a worktree's folder (protocol 10,
//! ADR 0027): every path checked by `vessel_workspace::paths` against the
//! root resolved for this call.
//!
//! [Decision Log]
//! - `file.find` keeps one running search per caller and key: a newer one
//!   cancels the older one, which answers `superseded` (a page typing a
//!   query sends one search per key stroke; only the last one matters).
//!   The caller is told apart by its principal's debug form, which names
//!   the device, agent session or origin; two connections of one device
//!   share their keys, as the device's own page does.
//! - Mutation constructors capture the original registered root before the
//!   service admits their owned future. The existing retained file task keeps
//!   its cache invalidation guard through physical disk completion, not just
//!   the caller's wait; no second file worker pool is introduced.

mod mutations;

use std::sync::atomic::Ordering;
use std::sync::{Arc, PoisonError};

use tokio_util::sync::CancellationToken;
use vessel_proto::workspace::{
    DirListing, FileRead, FileVersion, FindQuery, FoundFiles, GrepCase, GrepMode, GrepQuery,
    GrepResult, RelPath, SearchKey, WorkspaceRoot, WriteExpect,
};
use vessel_workspace::files;

use super::{WorkspaceError, Workspaces};
use crate::transfers::Holder;

impl Workspaces {
    /// `file.list`.
    ///
    /// # Errors
    /// `not_found` for a path that is not there or leads out.
    pub(crate) async fn list_dir(
        &self,
        root: WorkspaceRoot,
        dir: &RelPath,
        ignored: bool,
    ) -> Result<DirListing, WorkspaceError> {
        let resolved = self.resolve(root).await?;
        let listing = match (&self.git, resolved.git) {
            (Some(git), true) => files::list_marked(git, &resolved.root, dir, ignored).await?,
            _ => files::list(&resolved.root, dir).await?,
        };
        Ok(listing)
    }

    /// `file.read`.
    ///
    /// # Errors
    /// As [`list_dir`](Self::list_dir).
    pub(crate) async fn read_file(
        &self,
        root: WorkspaceRoot,
        path: &RelPath,
        offset: u64,
        limit: Option<u32>,
    ) -> Result<FileRead, WorkspaceError> {
        let resolved = self.resolve(root).await?;
        Ok(files::read(&resolved.root, path, offset, limit).await?)
    }

    /// `file.write`.
    ///
    /// # Errors
    /// `conflict` when `expect` does not hold.
    pub(crate) fn write_file(
        self: &Arc<Self>,
        root: WorkspaceRoot,
        path: &RelPath,
        text: String,
        expect: WriteExpect,
    ) -> impl Future<Output = Result<FileVersion, WorkspaceError>> + Send + 'static {
        let path = path.clone();
        self.mutating_file(root, move |opened| async move {
            files::write(&opened, &path, text, expect).await
        })
    }

    /// `file.create_dir`.
    ///
    /// # Errors
    /// `conflict` when the path exists.
    pub(crate) fn create_dir(
        self: &Arc<Self>,
        root: WorkspaceRoot,
        path: &RelPath,
    ) -> impl Future<Output = Result<(), WorkspaceError>> + Send + 'static {
        let path = path.clone();
        self.mutating_file(root, move |opened| async move {
            files::create_dir(&opened, &path).await
        })
    }

    /// `file.rename`.
    ///
    /// # Errors
    /// `conflict` when `to` exists.
    pub(crate) fn rename_path(
        self: &Arc<Self>,
        root: WorkspaceRoot,
        from: &RelPath,
        to: &RelPath,
    ) -> impl Future<Output = Result<(), WorkspaceError>> + Send + 'static {
        let (from, to) = (from.clone(), to.clone());
        self.mutating_file(root, move |opened| async move {
            files::rename(&opened, &from, &to).await
        })
    }

    /// `file.delete`.
    ///
    /// # Errors
    /// `conflict` for a folder that is not empty without `recursive`.
    pub(crate) fn delete_path(
        self: &Arc<Self>,
        root: WorkspaceRoot,
        path: &RelPath,
        recursive: bool,
    ) -> impl Future<Output = Result<(), WorkspaceError>> + Send + 'static {
        let path = path.clone();
        self.mutating_file(root, move |opened| async move {
            files::delete(&opened, &path, recursive).await
        })
    }

    /// `file.find`: by name, a newer search of the same caller and key
    /// replacing this one.
    ///
    /// # Errors
    /// As [`resolve`](Self::resolve).
    pub(crate) async fn find_files(
        &self,
        caller: Holder,
        root: WorkspaceRoot,
        query: &FindQuery,
        key: Option<SearchKey>,
        limit: Option<u32>,
    ) -> Result<FoundFiles, WorkspaceError> {
        let resolved = self.resolve(root).await?;
        let cancel = CancellationToken::new();
        let mine = self.search_count.fetch_add(1, Ordering::Relaxed);
        let slot = key.map(|key| (caller, key.0));
        if let Some(slot) = &slot {
            let older = self
                .searches
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(*slot, (mine, cancel.clone()));
            if let Some((_, older)) = older {
                older.cancel();
            }
        }
        let index = self.indexes.get(self.git.as_ref(), &resolved.root).await;
        let found = if cancel.is_cancelled() {
            superseded(&index)
        } else {
            let query = query.as_str().to_owned();
            let searching = Arc::clone(&index);
            let token = cancel.clone();
            tokio::task::spawn_blocking(move || searching.search(&query, limit, &token))
                .await
                .unwrap_or_else(|_panicked| superseded(&index))
        };
        if let Some(slot) = slot {
            let mut searches = self.searches.lock().unwrap_or_else(PoisonError::into_inner);
            // Only this search's own entry goes; a newer one stays.
            if searches
                .get(&slot)
                .is_some_and(|(running, _)| *running == mine)
            {
                drop(searches.remove(&slot));
            }
        }
        Ok(found)
    }

    /// `file.grep`.
    ///
    /// # Errors
    /// `unsupported` without git (it searches with `git grep`).
    pub(crate) async fn search_files(
        &self,
        root: WorkspaceRoot,
        query: &GrepQuery,
        mode: GrepMode,
        case: GrepCase,
        dir: Option<&RelPath>,
        limit: Option<u32>,
    ) -> Result<GrepResult, WorkspaceError> {
        let resolved = self.resolve(root).await?;
        let git = self.git()?;
        if let Some(dir) = dir {
            // The folder searched in is checked as any path is.
            let (opened, dir) = (resolved.root.clone(), dir.clone());
            let _folder = tokio::task::spawn_blocking(move || opened.existing(&dir))
                .await
                .map_err(|_panicked| {
                    WorkspaceError::Io("resolving the folder stopped".to_owned())
                })??;
        }
        Ok(git
            .grep(
                resolved.root.path(),
                resolved.git,
                query,
                mode,
                case,
                dir,
                limit,
            )
            .await?)
    }
}

fn superseded(index: &vessel_workspace::find::NameIndex) -> FoundFiles {
    FoundFiles {
        matches: Vec::new(),
        indexed: u32::try_from(index.len()).unwrap_or(u32::MAX),
        complete: false,
        superseded: true,
    }
}
