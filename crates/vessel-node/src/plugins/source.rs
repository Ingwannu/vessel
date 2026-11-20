//! Paseo's plugin source strings, read as this node takes them (ADR 0115).
//!
//! [Decision Log]
//! - Purpose: `plugin.install`'s `source` is the string a person types, as
//!   Paseo's `parsePluginSourceReference`
//!   (`packages/protocol/src/plugin-source-reference.ts:9-47`) reads it:
//!   `npm:`, `github:` and `git:` prefixes are managed sources; a string
//!   starting with `/`, `~`, `.`, a drive or `\\` is a directory, with an
//!   optional `:<portable relative subpath>`; anything else (an URL,
//!   `owner/repo`, a registry name) is managed.
//! - This protocol installs directories only; a managed source is refused
//!   by name until managed sources land (ADR 0115, P4), never fetched.
//! - The node takes absolute paths and `~` / `~/…` (ADR 0113's expansion,
//!   the node user's home); `~user` and relative paths are refused, since
//!   the node has no caller's working directory (the CLI makes a relative
//!   path absolute before sending, as Paseo's CLI resolves it).

use std::path::{Path, PathBuf};

/// Why a source string is not one this node installs.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum SourceError {
    /// Empty or only white space.
    #[error("a plugin source is required")]
    Empty,
    /// A Git, npm or registry source.
    #[error(
        "this node installs plugins from directories only; Git, npm and registry sources are not supported yet"
    )]
    Managed,
    /// `./x`, `../x`, `x` and the like.
    #[error("a plugin directory must be an absolute path or start with ~")]
    Relative,
    /// `~user/x`.
    #[error("only ~ and ~/ stand for the home folder; ~user is not supported")]
    OtherHome,
    /// `--ref` with a directory, Paseo's refusal.
    #[error("--ref is only valid for Git sources")]
    Reference,
    /// The resolved path is not an existing directory.
    #[error("plugin directory does not exist: {0}")]
    Missing(String),
}

/// An absolute plugin directory, as the person named it (`~` expanded; no
/// link resolved, as Paseo keeps `path.resolve`'s result).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Directory(PathBuf);

impl Directory {
    /// The path.
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    /// Take a stored absolute path back (the installations file).
    pub(crate) fn stored(path: PathBuf) -> Option<Self> {
        path.is_absolute().then_some(Self(path))
    }

    pub(crate) fn into_path(self) -> PathBuf {
        self.0
    }
}

/// Read `source` (with `reference`, a Git ref) as a plugin directory on
/// this node, `~` meaning `home`. Checks that the directory exists.
pub(crate) fn directory(
    source: &str,
    reference: Option<&str>,
    home: &Path,
) -> Result<Directory, SourceError> {
    let source = source.trim();
    if source.is_empty() {
        return Err(SourceError::Empty);
    }
    if ["npm:", "github:"]
        .iter()
        .any(|prefix| source.starts_with(prefix))
        || (source.starts_with("git:") && !source.starts_with("git://"))
        || !looks_like_directory(source)
    {
        return Err(SourceError::Managed);
    }
    if reference.is_some() {
        return Err(SourceError::Reference);
    }
    let (base, subpath) = split_subpath(source);
    let mut path = expand(base, home)?;
    if let Some(subpath) = subpath {
        path.push(subpath);
    }
    if !path.is_dir() {
        return Err(SourceError::Missing(path.display().to_string()));
    }
    Ok(Directory(path))
}

/// Read a plain path (`plugin.inspect`): absolute or `~`.
pub(crate) fn path(path: &str, home: &Path) -> Result<PathBuf, SourceError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(SourceError::Empty);
    }
    expand(path, home)
}

/// Paseo's directory test: `.`/`..` alone or before a separator, `/`, `~`,
/// a drive letter or a UNC prefix.
fn looks_like_directory(source: &str) -> bool {
    let mut characters = source.chars();
    let first = characters.next();
    let second = characters.next();
    let third = characters.next();
    let dots = source == "."
        || source == ".."
        || source.starts_with("./")
        || source.starts_with(".\\")
        || source.starts_with("../")
        || source.starts_with("..\\");
    let drive = matches!(
        (first, second, third),
        (Some(letter), Some(':'), Some('/' | '\\')) if letter.is_ascii_alphabetic()
    );
    dots || source.starts_with('/')
        || source.starts_with('~')
        || drive
        || source.starts_with("\\\\")
}

/// Paseo's `splitPluginPath` for a directory: the part after the last `:`
/// is a subpath when it is a portable relative path; a drive's own colon
/// (`C:\x`) is not a separator.
fn split_subpath(source: &str) -> (&str, Option<&str>) {
    let Some((base, subpath)) = source.rsplit_once(':') else {
        return (source, None);
    };
    let drive_colon = base.len() == 1 && base.chars().all(|letter| letter.is_ascii_alphabetic());
    if drive_colon || base.is_empty() || !portable_relative(subpath) {
        return (source, None);
    }
    (base, Some(subpath))
}

/// A path relative to a repository on every system: segments of letters,
/// digits, `.`, `_`, `-` and `@`, no `.` or `..` segment, `/` between them.
fn portable_relative(path: &str) -> bool {
    !path.is_empty()
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@'))
        })
}

/// `~` and `~/…` under `home`; an absolute path as it is.
fn expand(path: &str, home: &Path) -> Result<PathBuf, SourceError> {
    if path == "~" {
        return Ok(home.to_path_buf());
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return Ok(home.join(rest));
    }
    if path.starts_with('~') {
        return Err(SourceError::OtherHome);
    }
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(SourceError::Relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_sources_are_refused_by_name_and_never_fetched() {
        let home = Path::new("/home/someone");
        for source in [
            "npm:@scope/plugin",
            "github:owner/repo",
            "git:owner/repo",
            "git://example.com/x.git",
            "https://example.com/x.git",
            "owner/repo",
            "owner/slug",
        ] {
            assert_eq!(
                directory(source, None, home),
                Err(SourceError::Managed),
                "{source}"
            );
        }
    }

    #[test]
    fn directories_take_tilde_absolute_and_a_portable_subpath() {
        let home = tempfile::tempdir().unwrap();
        let plugin = home.path().join("repo/plugins/theme");
        std::fs::create_dir_all(&plugin).unwrap();
        let by_tilde = directory("~/repo/plugins/theme", None, home.path()).unwrap();
        assert_eq!(by_tilde.path(), plugin);
        let absolute = format!("{}:plugins/theme", home.path().join("repo").display());
        assert_eq!(
            directory(&absolute, None, home.path()).unwrap().path(),
            plugin
        );
        assert_eq!(
            directory("~/repo:plugins/theme", None, home.path())
                .unwrap()
                .path(),
            plugin
        );
        // A subpath that is not portable is part of the path.
        assert!(matches!(
            directory("~/repo:../x", None, home.path()),
            Err(SourceError::Missing(_))
        ));
    }

    #[test]
    fn relative_other_homes_refs_and_missing_directories_are_refused() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(
            directory("./plugin", None, home.path()),
            Err(SourceError::Relative)
        );
        assert_eq!(
            directory("~other/plugin", None, home.path()),
            Err(SourceError::OtherHome)
        );
        assert_eq!(
            directory("~", Some("main"), home.path()),
            Err(SourceError::Reference)
        );
        assert_eq!(directory("  ", None, home.path()), Err(SourceError::Empty));
        assert!(matches!(
            directory("~/missing", None, home.path()),
            Err(SourceError::Missing(_))
        ));
        assert_eq!(
            directory("~", None, home.path()).unwrap().path(),
            home.path()
        );
    }
}
