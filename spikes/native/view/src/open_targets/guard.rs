//! What may be handed to an editor or a file manager (ADR 0123, the
//! coordinator's conditions): absolute canonical paths that exist, the
//! shown workspace root or under it, so no argument made from them can
//! begin with `-`.

use std::path::{Path, PathBuf};

use super::Probe;

/// What the person asked to open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The workspace shown (the session's folder).
    pub root: PathBuf,
    /// The folder to open: the root or a folder under it.
    pub folder: PathBuf,
    /// A file under the root to open, or the entry to reveal.
    pub file: Option<PathBuf>,
    /// The file's line, from 1.
    pub line: Option<u32>,
    /// The line's column, from 1.
    pub column: Option<u32>,
}

/// A request whose paths passed: canonical, existing, under the root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// The folder, canonical.
    pub folder: PathBuf,
    /// The file, canonical.
    pub file: Option<PathBuf>,
    /// The file's line.
    pub line: Option<u32>,
    /// The line's column.
    pub column: Option<u32>,
}

/// Why a request was refused before anything was started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// A path is not absolute (Paseo's "must be an absolute local path").
    NotAbsolute(PathBuf),
    /// A path does not exist here (Paseo's "Path does not exist").
    Missing(PathBuf),
    /// A path resolves outside the workspace root (a link out of it).
    Outside(PathBuf),
}

/// `request`'s paths, resolved and checked.
///
/// # Errors
///
/// A path that is not absolute, does not exist here, resolves outside the
/// root, or would read as an option.
pub fn checked(request: &Request, probe: &dyn Probe) -> Result<Checked, Refused> {
    let root = resolve(&request.root, probe)?;
    let inside = |path: &Path| -> Result<PathBuf, Refused> {
        let resolved = resolve(path, probe)?;
        if resolved.starts_with(&root) {
            Ok(resolved)
        } else {
            Err(Refused::Outside(path.to_path_buf()))
        }
    };
    let folder = inside(&request.folder)?;
    let file = request.file.as_deref().map(inside).transpose()?;
    Ok(Checked {
        folder,
        file,
        line: request.line.filter(|line| *line > 0),
        column: request.column.filter(|column| *column > 0),
    })
}

/// `path` absolute, canonical and not read as an option.
fn resolve(path: &Path, probe: &dyn Probe) -> Result<PathBuf, Refused> {
    if !path.is_absolute() {
        return Err(Refused::NotAbsolute(path.to_path_buf()));
    }
    let resolved = probe
        .canonical(path)
        .map(plain)
        .map_err(|_missing| Refused::Missing(path.to_path_buf()))?;
    // Absolute on every platform means `/…`, `C:\…` or `\\…`: never `-`.
    // Checked anyway, so an argument can never be taken as an option.
    if !resolved.is_absolute() || resolved.to_string_lossy().starts_with('-') {
        return Err(Refused::NotAbsolute(path.to_path_buf()));
    }
    Ok(resolved)
}

/// Windows names a canonical drive path `\\?\C:\…`; editors and Explorer
/// take `C:\…`. Other forms (`\\?\UNC\…`) are kept as they are.
fn plain(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy().into_owned();
    let Some(rest) = text.strip_prefix(r"\\?\") else {
        return path;
    };
    let mut chars = rest.chars();
    let drive = chars.next().is_some_and(|letter| letter.is_ascii_alphabetic())
        && chars.next() == Some(':')
        && chars.next() == Some('\\');
    if drive { PathBuf::from(rest) } else { path }
}

#[cfg(test)]
pub(super) fn plain_for_tests(path: &str) -> PathBuf {
    plain(PathBuf::from(path))
}
