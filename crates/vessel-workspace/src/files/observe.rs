//! Finite single-file observation watches a parent directory, so atomic
//! replacements remain visible without a recursive repository watch.
//! The request owns and drops its watcher on every completion path.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use notify::{RecursiveMode, Watcher as _};
use tokio::sync::watch;
use vessel_proto::workspace::{RelPath, panel_files::ObservedFile};

use super::{FileError, binary::observed, blocking};
use crate::paths::Root;

/// No OS watch outlives this request's finite heartbeat interval.
pub const OBSERVE_HEARTBEAT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
enum Signal {
    Quiet,
    Dirty,
    Failed(Arc<notify::Error>),
}

struct Observation {
    watcher: notify::RecommendedWatcher,
    changed: watch::Receiver<Signal>,
}

impl Observation {
    async fn start(root: &Root, path: &RelPath) -> Result<Self, FileError> {
        let (root, path) = (root.clone(), path.clone());
        blocking(move || {
            let mut targets = vec![path.under(root.path())];
            let actual = match root.existing(&path) {
                Ok(actual) => Some(actual),
                Err(crate::paths::PathError::NotFound(_)) => None,
                Err(error) => return Err(error.into()),
            };
            match root.entry(&path) {
                Ok(entry) => targets.push(entry.path),
                Err(crate::paths::PathError::NotFound(_)) => {}
                Err(error) => return Err(error.into()),
            }
            let mut parent = path.parent().ok_or(crate::paths::PathError::Root)?;
            let directory = loop {
                match root.existing(&parent) {
                    Ok(directory) => break directory,
                    Err(crate::paths::PathError::NotFound(_)) => {
                        parent = parent.parent().ok_or(crate::paths::PathError::Root)?;
                    }
                    Err(error) => return Err(error.into()),
                }
            };
            let actual_parent = actual
                .as_ref()
                .and_then(|path| path.parent())
                .map(std::path::Path::to_path_buf);
            if let Some(actual) = actual {
                targets.push(actual);
            }
            let (sender, changed) = watch::channel(Signal::Quiet);
            let mut watcher =
                notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                    let signal = match event {
                        Ok(event)
                            if !event.kind.is_access()
                                && targets.iter().any(|target| relevant(&event.paths, target)) =>
                        {
                            Signal::Dirty
                        }
                        Ok(_) => return,
                        Err(error) => Signal::Failed(Arc::new(error)),
                    };
                    let _sent = sender.send_if_modified(|current| {
                        // A later successful notification must never erase
                        // overflow/failure evidence before the reader sees it.
                        if matches!(current, Signal::Failed(_)) {
                            return false;
                        }
                        *current = signal;
                        true
                    });
                })
                .map_err(watch_error)?;
            if let Some(parent) = actual_parent.filter(|parent| parent != &directory) {
                watcher
                    .watch(&parent, RecursiveMode::NonRecursive)
                    .map_err(watch_error)?;
            }
            watcher
                .watch(&directory, RecursiveMode::NonRecursive)
                .map_err(watch_error)?;
            Ok(Self { watcher, changed })
        })
        .await
    }

    async fn wait(&mut self) -> Result<(), FileError> {
        if tokio::time::timeout(OBSERVE_HEARTBEAT, self.changed.changed())
            .await
            .is_ok_and(|result| result.is_err())
        {
            return Err(FileError::Task);
        }
        match self.changed.borrow_and_update().clone() {
            Signal::Quiet | Signal::Dirty => Ok(()),
            Signal::Failed(error) => Err(FileError::Io(std::io::Error::other(error))),
        }
    }

    async fn after(
        &mut self,
        root: &Root,
        path: &RelPath,
        after: &ObservedFile,
    ) -> Result<ObservedFile, FileError> {
        drop(self.changed.borrow_and_update());
        let current = observed(root, path).await?;
        if &current != after {
            return Ok(current);
        }
        self.wait().await?;
        observed(root, path).await
    }
}

fn watch_error(error: notify::Error) -> FileError {
    FileError::Io(std::io::Error::other(error))
}

fn relevant(paths: &[PathBuf], target: &std::path::Path) -> bool {
    paths.is_empty() || paths.iter().any(|path| target.starts_with(path))
}

/// Snapshot a file, or wait finitely for a filesystem invalidation.
///
/// # Errors
/// Watch failures are visible errors, never a claim that the file is current.
pub async fn observe(
    root: &Root,
    path: &RelPath,
    after: Option<&ObservedFile>,
) -> Result<ObservedFile, FileError> {
    let Some(after) = after else {
        return observed(root, path).await;
    };
    let mut observation = Observation::start(root, path).await?;
    let result = observation.after(root, path, after).await;
    // Platform watcher destructors can wait for their worker thread.
    tokio::task::spawn_blocking(move || drop(observation.watcher))
        .await
        .map_err(|_invalid| FileError::Task)?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parent_watch_survives_atomic_replace_and_deletion_without_sleep() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::open(temp.path()).unwrap();
        let path: RelPath = "file.rs".parse().unwrap();
        std::fs::write(temp.path().join("file.rs"), "before").unwrap();
        let before = observed(&root, &path).await.unwrap();
        let mut watching = Observation::start(&root, &path).await.unwrap();
        std::fs::write(temp.path().join("replacement"), "after").unwrap();
        std::fs::rename(temp.path().join("replacement"), temp.path().join("file.rs")).unwrap();
        watching.wait().await.unwrap();
        assert_ne!(observed(&root, &path).await.unwrap(), before);
        std::fs::remove_file(temp.path().join("file.rs")).unwrap();
        watching.wait().await.unwrap();
        assert_eq!(observed(&root, &path).await.unwrap(), ObservedFile::Missing);
    }
}
