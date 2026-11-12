//! Keeping every path a client names inside its folder (ADR 0024).
//!
//! [Decision Log]
//! - Purpose: a file or git call names a [`RelPath`] inside a project or a
//!   worktree. Its shape is checked where it is decoded (no `..`, no
//!   absolute path, no drive, no `.git`); what the disk says is checked
//!   here: a symbolic link can still lead out.
//! - Choice: a [`Root`] holds its folder resolved once (links followed,
//!   `std::fs::canonicalize`). A path to read is joined part by part,
//!   resolved, and must still be inside the root and outside its `.git`.
//!   A path to write or create resolves only its parent (it may not exist
//!   yet) under the same rule, and refuses a last part that is a symbolic
//!   link, so a write never lands where a link points. An existing target
//!   is resolved too, which also catches a Windows short name (`GIT~1`)
//!   standing for `.git`.
//! - Alternatives: rejecting every symbolic link (repositories use links
//!   inside themselves, `node_modules/.bin` for one); `openat2` with
//!   `RESOLVE_BENEATH` (Linux only, and through `unsafe` or a new crate).
//! - Cost: checks, not a sandbox. Between the check and the open a process
//!   of the same user can swap a folder for a link (the time-of-check race
//!   ADR 0016 accepts for the language servers); a hostile local process
//!   already has the user's files.
//!
//! Everything here touches the disk and blocks: call it from a blocking
//! task.

use std::io;
use std::path::{Path, PathBuf};

use vessel_proto::workspace::RelPath;

/// A folder that calls are kept inside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Root {
    resolved: PathBuf,
    /// The repository's own folder when it lies inside the root (a main
    /// checkout's `.git`); a linked worktree's lies elsewhere.
    git_dir: Option<PathBuf>,
}

/// Why a path cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum PathError {
    /// It leads outside the root (through a symbolic link).
    #[error("`{0}` leads outside its folder")]
    Outside(RelPath),
    /// It leads into the repository's `.git`.
    #[error("`{0}` is inside the repository's `.git`")]
    GitDir(RelPath),
    /// Nothing is there (or its folder is missing, for a target).
    #[error("`{0}` does not exist")]
    NotFound(RelPath),
    /// The call needs a path below the root, not the root itself.
    #[error("the folder itself cannot be the target")]
    Root,
    /// The last part is a symbolic link, which a write never follows.
    #[error("`{0}` is a symbolic link")]
    Link(RelPath),
    /// A name this platform reads as something else (`:` on Windows names
    /// a stream of another file).
    #[error("`{0}` cannot be used on this machine")]
    Platform(RelPath),
    /// The root is not a folder, or the disk refused.
    #[error("cannot read the folder")]
    Io(#[source] io::Error),
}

impl Root {
    /// The folder at `path`, resolved.
    ///
    /// # Errors
    /// [`PathError::Io`] when it does not exist or is not a folder.
    pub fn open(path: &Path) -> Result<Self, PathError> {
        let resolved = std::fs::canonicalize(path).map_err(PathError::Io)?;
        if !resolved.is_dir() {
            return Err(PathError::Io(io::Error::new(
                io::ErrorKind::NotADirectory,
                "not a folder",
            )));
        }
        let git_dir = std::fs::canonicalize(resolved.join(".git"))
            .ok()
            .filter(|dir| dir.is_dir() && dir.starts_with(&resolved));
        Ok(Self { resolved, git_dir })
    }

    /// The folder, resolved.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.resolved
    }

    /// An existing path to read: links followed, and still inside.
    ///
    /// # Errors
    /// [`PathError::NotFound`], [`PathError::Outside`],
    /// [`PathError::GitDir`], [`PathError::Platform`].
    pub fn existing(&self, path: &RelPath) -> Result<PathBuf, PathError> {
        platform(path)?;
        let resolved = std::fs::canonicalize(path.under(&self.resolved)).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                PathError::NotFound(path.clone())
            } else {
                PathError::Io(error)
            }
        })?;
        self.inside(path, &resolved)?;
        Ok(resolved)
    }

    /// A path to create, rename or remove: its folder resolved and inside;
    /// its last part as it is, a symbolic link included (removing or moving
    /// a link acts on the link, never on what it points to).
    ///
    /// # Errors
    /// [`PathError::Root`] for the root itself, [`PathError::NotFound`] for a
    /// missing folder, and the others as [`existing`](Self::existing).
    pub fn entry(&self, path: &RelPath) -> Result<Target, PathError> {
        let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
            return Err(PathError::Root);
        };
        platform(path)?;
        let folder = self.existing(&parent).map_err(|error| {
            if let PathError::NotFound(_) = error {
                PathError::NotFound(path.clone())
            } else {
                error
            }
        })?;
        if !folder.is_dir() {
            return Err(PathError::NotFound(path.clone()));
        }
        let full = folder.join(name);
        let kind = match std::fs::symlink_metadata(&full) {
            Ok(metadata) if metadata.file_type().is_symlink() => Kind::Link,
            Ok(_) => Kind::Other,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Kind::Missing,
            Err(error) => return Err(PathError::Io(error)),
        };
        if kind == Kind::Other {
            // A Windows short name (`GIT~1`) is caught here: it resolves to
            // the long name it stands for.
            let resolved = std::fs::canonicalize(&full).map_err(PathError::Io)?;
            self.inside(path, &resolved)?;
        }
        Ok(Target {
            path: full,
            exists: kind != Kind::Missing,
            link: kind == Kind::Link,
        })
    }

    /// A file to write: as [`entry`](Self::entry), but a symbolic link is
    /// followed to the file it names when that is inside the root (an
    /// editor saving `CLAUDE.md` that links to `AGENTS.md` writes
    /// `AGENTS.md`); the link itself is never replaced.
    ///
    /// # Errors
    /// As [`entry`](Self::entry); [`PathError::Link`] for a link that leads
    /// nowhere, [`PathError::Outside`] for one that leads out.
    pub fn write_target(&self, path: &RelPath) -> Result<Target, PathError> {
        let entry = self.entry(path)?;
        if !entry.link {
            return Ok(entry);
        }
        let resolved = std::fs::canonicalize(&entry.path)
            .map_err(|_dangling| PathError::Link(path.clone()))?;
        self.inside(path, &resolved)?;
        Ok(Target {
            path: resolved,
            exists: true,
            link: false,
        })
    }

    /// Refuse `resolved` (what `path` led to) outside the root, or under any
    /// `.git` below it (the repository's own, or a nested repository's
    /// reached through a link).
    fn inside(&self, path: &RelPath, resolved: &Path) -> Result<(), PathError> {
        let Ok(below) = resolved.strip_prefix(&self.resolved) else {
            return Err(PathError::Outside(path.clone()));
        };
        let git = below.components().any(|part| {
            part.as_os_str()
                .to_str()
                .is_some_and(|name| name.eq_ignore_ascii_case(".git"))
        });
        if git
            || self
                .git_dir
                .as_ref()
                .is_some_and(|dir| resolved.starts_with(dir))
        {
            return Err(PathError::GitDir(path.clone()));
        }
        Ok(())
    }
}

/// What is at a path, not following a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Missing,
    Link,
    Other,
}

/// A checked place to act on: its folder is inside the root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The full path (its folder resolved, its name as given; for
    /// [`Root::write_target`] through a link, the file the link names).
    pub path: PathBuf,
    /// Whether something is there now.
    pub exists: bool,
    /// Whether that something is a symbolic link.
    pub link: bool,
}

/// On Windows a `:` in a name selects an alternate data stream of another
/// file; refused there.
#[cfg(windows)]
fn platform(path: &RelPath) -> Result<(), PathError> {
    if path.as_str().contains(':') {
        return Err(PathError::Platform(path.clone()));
    }
    Ok(())
}

#[cfg(not(windows))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the same signature as the Windows check, which can fail"
)]
const fn platform(_: &RelPath) -> Result<(), PathError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(text: &str) -> RelPath {
        text.parse().unwrap()
    }

    /// A root with `src/a.txt`, a `.git` folder, and a folder outside it.
    fn setup() -> (tempfile::TempDir, Root, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let outside = temp.path().join("outside");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(root.join("src/a.txt"), "a").unwrap();
        std::fs::write(root.join(".git/config"), "x").unwrap();
        std::fs::write(outside.join("secret"), "s").unwrap();
        let opened = Root::open(&root).unwrap();
        (temp, opened, outside)
    }

    #[test]
    fn a_path_inside_resolves_and_a_missing_one_says_so() {
        let (_temp, root, _) = setup();
        let found = root.existing(&rel("src/a.txt")).unwrap();
        assert!(found.starts_with(root.path()));
        assert!(matches!(
            root.existing(&rel("src/nope")),
            Err(PathError::NotFound(_))
        ));
        assert_eq!(root.existing(&RelPath::root()).unwrap(), root.path());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_out_of_the_root_is_refused_for_reads_and_writes() {
        use std::os::unix::fs::symlink;
        let (_temp, root, outside) = setup();
        symlink(&outside, root.path().join("out")).unwrap();
        symlink(outside.join("secret"), root.path().join("secret")).unwrap();
        // Reading through a link to a folder or a file outside.
        assert!(matches!(
            root.existing(&rel("out/secret")),
            Err(PathError::Outside(_))
        ));
        assert!(matches!(
            root.existing(&rel("secret")),
            Err(PathError::Outside(_))
        ));
        // Writing into a folder that is a link outside.
        assert!(matches!(
            root.write_target(&rel("out/new.txt")),
            Err(PathError::Outside(_))
        ));
        assert!(matches!(
            root.entry(&rel("out/new.txt")),
            Err(PathError::Outside(_))
        ));
        // Writing through a link out is refused; through one inside, it
        // writes the file the link names and leaves the link.
        assert!(matches!(
            root.write_target(&rel("secret")),
            Err(PathError::Outside(_))
        ));
        symlink(root.path().join("src/a.txt"), root.path().join("inner")).unwrap();
        let through = root.write_target(&rel("inner")).unwrap();
        assert_eq!(through.path, root.path().join("src").join("a.txt"));
        assert!(!through.link);
        // Removing or moving a link acts on the link itself.
        let link = root.entry(&rel("secret")).unwrap();
        assert!(link.link && link.exists);
        assert_eq!(link.path, root.path().join("secret"));
        // A link that leads nowhere is not written through.
        symlink(root.path().join("gone"), root.path().join("dangling")).unwrap();
        assert!(matches!(
            root.write_target(&rel("dangling")),
            Err(PathError::Link(_))
        ));
        // A link inside that stays inside reads fine.
        assert!(root.existing(&rel("inner")).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_into_git_is_refused() {
        use std::os::unix::fs::symlink;
        let (_temp, root, _) = setup();
        symlink(root.path().join(".git"), root.path().join("g")).unwrap();
        assert!(matches!(
            root.existing(&rel("g/config")),
            Err(PathError::GitDir(_))
        ));
        assert!(matches!(
            root.entry(&rel("g/hooks")),
            Err(PathError::GitDir(_))
        ));
        // A nested repository's `.git`, reached through a link inside.
        std::fs::create_dir_all(root.path().join("vendor/lib/.git")).unwrap();
        std::fs::write(root.path().join("vendor/lib/.git/config"), "x").unwrap();
        symlink(
            root.path().join("vendor/lib/.git"),
            root.path().join("nested"),
        )
        .unwrap();
        assert!(matches!(
            root.existing(&rel("nested/config")),
            Err(PathError::GitDir(_))
        ));
    }

    #[test]
    fn a_target_needs_its_folder_and_is_never_the_root() {
        let (_temp, root, _) = setup();
        let target = root.write_target(&rel("src/new.txt")).unwrap();
        assert!(!target.exists);
        assert_eq!(target.path, root.path().join("src").join("new.txt"));
        assert!(root.write_target(&rel("src/a.txt")).unwrap().exists);
        assert!(matches!(
            root.write_target(&rel("missing/new.txt")),
            Err(PathError::NotFound(_))
        ));
        assert!(matches!(root.entry(&RelPath::root()), Err(PathError::Root)));
        assert!(matches!(
            root.entry(&rel("src/a.txt/below")),
            Err(PathError::NotFound(_))
        ));
    }

    #[test]
    fn climbing_and_absolute_paths_never_get_here() {
        // The shape is refused where it is decoded, so no `RelPath` can say
        // these; checked here so this module's guarantees are stated whole.
        for text in [
            "../outside/secret",
            "/etc/passwd",
            "src/../../outside",
            ".git/config",
        ] {
            assert!(text.parse::<RelPath>().is_err(), "{text}");
        }
    }
}
