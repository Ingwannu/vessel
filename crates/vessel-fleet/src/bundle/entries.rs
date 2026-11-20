//! Reading a skills directory's entries while a bundle is made.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use vessel_proto::BundlePath;

use super::BundleError;

/// The entries of `dir` with UTF-8 names, by name.
pub(super) fn sorted_entries(dir: &Path) -> Result<Vec<(String, PathBuf)>, BundleError> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).map_err(|source| read_error(dir, source))? {
        let entry = entry.map_err(|source| read_error(dir, source))?;
        if let Ok(name) = entry.file_name().into_string() {
            entries.push((name, entry.path()));
        }
    }
    entries.sort();
    Ok(entries)
}

pub(super) fn bundle_path(text: &str) -> Option<BundlePath> {
    text.parse().ok()
}

pub(super) fn read_error(path: &Path, source: io::Error) -> BundleError {
    BundleError::Read {
        path: path.to_path_buf(),
        source,
    }
}
