//! A new empty project directory, Paseo's New directory (ADR 0113), and
//! `~` in the project paths a person picks: this call's parent,
//! `project.add`'s path and `project.clone`'s parent.
//!
//! [Decision Log]
//! - Purpose: the Add project sheet's New directory (TRACKER 242-243) as
//!   Paseo's `project-directory-service.ts`: one empty folder, made a
//!   project, no Git; and the sheet's `~` paths (`~/dev`, a typed `~/x`)
//!   read as the home, as Paseo's server reads them.
//! - Constraint: every root is checked against the refused ones
//!   (`policy::check_root`); the name is one plain component, checked when
//!   the request is decoded (`DirectoryName`); `project.init` makes parents
//!   and a repository, which Paseo's New directory does not.
//! - Choice: resolve the parent (`~` expanded), make the folder with one
//!   non-recursive `create_dir`, register it without Git; when registering
//!   fails, remove it again (`remove_dir`, empty only) and answer the
//!   failure, naming a failed removal. `~` is expanded only alone or before
//!   `/` (`~user` stays relative, so refused), to the home `folder.suggest`
//!   walks (`Places::home`).
//! - Cost: the errors are the node's usual kinds (`invalid_request`,
//!   `conflict`, `internal` with the disk's words) rather than Paseo's own
//!   codes; the sheet shows the message, as Paseo's shows `payload.error`.

use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use vessel_proto::registry::DirectoryName;
use vessel_proto::workspace::{ProjectInfo, ProjectState};

use super::adding::existing_folder;
use crate::workspace::{WorkspaceError, Workspaces, policy};

/// `path` with a leading `~` (alone, or before `/`) as `home`; any other
/// path as given.
pub(super) fn expand_home(home: &Path, path: PathBuf) -> PathBuf {
    let mut parts = path.components();
    match parts.next() {
        Some(Component::Normal(first)) if first == "~" => home.join(parts.as_path()),
        _ => path,
    }
}

impl Workspaces {
    /// `path` with a leading `~` as the node user's home (ADR 0113).
    pub(super) fn home_expanded(&self, path: PathBuf) -> PathBuf {
        expand_home(&self.places.home, path)
    }

    /// Make the folder `name` in `parent` (absolute, or `~` for the home)
    /// and add it as a project without Git.
    ///
    /// # Errors
    /// `invalid_request` for a parent that is not an existing absolute
    /// folder or a refused root; `conflict` when the folder exists or past
    /// the projects' bound; `internal` when the disk refuses.
    pub(crate) async fn create_project_directory(
        self: &Arc<Self>,
        parent: PathBuf,
        name: DirectoryName,
    ) -> Result<ProjectInfo, WorkspaceError> {
        let parent = existing_folder(self.home_expanded(parent)).await?;
        let target = parent.join(name.as_str());
        // `DirectoryName` is one plain component; the join stays a child.
        if target.parent() != Some(parent.as_path()) {
            return Err(WorkspaceError::Invalid(format!(
                "`{name}` is not one folder name"
            )));
        }
        policy::check_root(&self.places, &target)?;
        let made = target.clone();
        tokio::task::spawn_blocking(move || std::fs::create_dir(&made))
            .await
            .map_err(|_panicked| WorkspaceError::Io("making the folder stopped".to_owned()))?
            .map_err(|error| made_error(&target, &error))?;
        match self
            .insert_project(target.clone(), false, None, ProjectState::Ready)
            .await
        {
            Ok(info) => Ok(info),
            Err(error) => Err(rolled_back(&target, error).await),
        }
    }
}

fn made_error(target: &Path, error: &std::io::Error) -> WorkspaceError {
    // `ErrorKind` grows with each release; two kinds have words, the
    // rest the disk's.
    let kind = error.kind();
    if kind == ErrorKind::AlreadyExists {
        WorkspaceError::Conflict(format!("`{}` already exists", target.display()))
    } else if kind == ErrorKind::NotFound {
        WorkspaceError::Invalid(format!(
            "the parent of `{}` does not exist",
            target.display()
        ))
    } else {
        WorkspaceError::Io(format!("making `{}`: {error}", target.display()))
    }
}

/// The folder just made removed again after registering failed: the
/// registration's error when it is gone, else both.
async fn rolled_back(target: &Path, error: WorkspaceError) -> WorkspaceError {
    let made = target.to_path_buf();
    let removed = tokio::task::spawn_blocking(move || std::fs::remove_dir(&made)).await;
    match removed {
        Ok(Ok(())) => error,
        Ok(Err(removal)) => WorkspaceError::Io(format!(
            "{error}; and `{}` could not be removed: {removal}",
            target.display()
        )),
        Err(_panicked) => WorkspaceError::Io(format!(
            "{error}; and removing `{}` stopped",
            target.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{expand_home, rolled_back};
    use crate::workspace::WorkspaceError;
    use std::path::{Path, PathBuf};

    #[tokio::test]
    async fn a_failed_registration_removes_the_folder_it_made() {
        let temp = tempfile::tempdir().unwrap();
        let made = temp.path().join("new");
        std::fs::create_dir(&made).unwrap();
        let error = rolled_back(&made, WorkspaceError::Conflict("full".to_owned())).await;
        assert!(
            matches!(error, WorkspaceError::Conflict(ref why) if why == "full"),
            "{error:?}"
        );
        assert!(!made.exists());
        // A folder something was put in stays, and the answer says so.
        std::fs::create_dir(&made).unwrap();
        std::fs::write(made.join("kept.txt"), "kept").unwrap();
        let error = rolled_back(&made, WorkspaceError::Conflict("full".to_owned())).await;
        assert!(
            matches!(error, WorkspaceError::Io(ref why) if why.contains("full") && why.contains("could not be removed")),
            "{error:?}"
        );
        assert!(made.join("kept.txt").exists());
    }

    #[test]
    fn only_a_leading_tilde_is_the_home() {
        let home = Path::new("/home/me");
        for (given, expanded) in [
            ("~", "/home/me"),
            ("~/", "/home/me"),
            ("~/dev/app", "/home/me/dev/app"),
            ("~user/dev", "~user/dev"),
            ("/srv/~/x", "/srv/~/x"),
            ("dev/~", "dev/~"),
            ("/abs", "/abs"),
        ] {
            assert_eq!(
                expand_home(home, PathBuf::from(given)),
                PathBuf::from(expanded),
                "{given}"
            );
        }
    }
}
