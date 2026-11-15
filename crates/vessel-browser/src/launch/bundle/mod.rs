//! Bounded read-only observation of the fixed candidate bundle, never a runtime grant.
mod hash;
#[cfg(test)]
mod tests;

use super::identity::LaunchError;
use hash::ManifestHash;
use std::fs;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

const MAX_BUNDLE_ENTRIES: usize = 4096;
const MAX_BUNDLE_DEPTH: usize = 16;
const MAX_BUNDLE_BYTES: u64 = 1024 * 1024 * 1024;
use super::identity::{ROOT_UID, WRITE_ACCESS};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Fingerprint([u8; 32]);
impl std::fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CandidateBundleFingerprint([observed])")
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Version {
    device: u64,
    inode: u64,
    length: u64,
    mode: u32,
    uid: u32,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}
impl Version {
    fn of(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            mode: metadata.mode(),
            uid: metadata.uid(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }
}
struct Entry {
    relative: PathBuf,
    version: Version,
}
/// Metadata checks detect changes after hashing; the observed digest is not an approved certificate.
pub(super) struct Bundle {
    entries: Vec<Entry>,
    fingerprint: Fingerprint,
}
impl Bundle {
    pub(super) fn inspect(root: &Path) -> Result<Self, LaunchError> {
        let mut entries = Vec::new();
        let mut remaining = MAX_BUNDLE_BYTES;
        let mut manifest = ManifestHash::new();
        collect(
            root,
            Path::new(""),
            0,
            &mut entries,
            &mut remaining,
            &mut manifest,
        )?;
        let bundle = Self {
            entries,
            fingerprint: manifest.finish(),
        };
        let _observed = bundle.verify_unchanged(root)?;
        Ok(bundle)
    }
    #[cfg(test)]
    pub(super) const fn fixture() -> Self {
        Self {
            entries: Vec::new(),
            fingerprint: Fingerprint([0; 32]),
        }
    }
    pub(super) fn verify_unchanged(&self, root: &Path) -> Result<Fingerprint, LaunchError> {
        for entry in &self.entries {
            let current =
                fs::symlink_metadata(root.join(&entry.relative)).map_err(LaunchError::Io)?;
            if Version::of(&current) != entry.version {
                return Err(LaunchError::Identity);
            }
        }
        Ok(self.fingerprint)
    }
}
fn collect(
    root: &Path,
    relative: &Path,
    depth: usize,
    entries: &mut Vec<Entry>,
    remaining: &mut u64,
    manifest: &mut ManifestHash,
) -> Result<(), LaunchError> {
    if depth > MAX_BUNDLE_DEPTH || entries.len() >= MAX_BUNDLE_ENTRIES {
        return Err(LaunchError::Identity);
    }
    let path = root.join(relative);
    let before = fs::symlink_metadata(&path).map_err(LaunchError::Io)?;
    if before.uid() != ROOT_UID || before.mode() & WRITE_ACCESS != 0 {
        return Err(LaunchError::Permissions);
    }
    if before.file_type().is_symlink() || (!before.is_dir() && !before.is_file()) {
        return Err(LaunchError::Identity);
    }
    let version = Version::of(&before);
    entries.push(Entry {
        relative: relative.to_owned(),
        version,
    });
    if before.is_file() {
        if before.len() > *remaining {
            return Err(LaunchError::Identity);
        }
        let mut file = fs::File::open(&path).map_err(LaunchError::Io)?;
        if Version::of(&file.metadata().map_err(LaunchError::Io)?) != version {
            return Err(LaunchError::Identity);
        }
        let digest = super::identity::digest_with_budget(&mut file, remaining)?;
        if Version::of(&file.metadata().map_err(LaunchError::Io)?) != version {
            return Err(LaunchError::Identity);
        }
        manifest.file(relative, before.mode(), before.len(), digest)?;
    } else {
        manifest.directory(relative, before.mode())?;
        let children = children(&path, entries.len())?;
        for name in children {
            collect(
                root,
                &relative.join(name),
                depth.checked_add(1).ok_or(LaunchError::Identity)?,
                entries,
                remaining,
                manifest,
            )?;
        }
    }
    if Version::of(&fs::symlink_metadata(path).map_err(LaunchError::Io)?) != version {
        return Err(LaunchError::Identity);
    }
    Ok(())
}
fn children(directory: &Path, already: usize) -> Result<Vec<std::ffi::OsString>, LaunchError> {
    let mut names = Vec::new();
    for entry in fs::read_dir(directory).map_err(LaunchError::Io)? {
        if already.saturating_add(names.len()) >= MAX_BUNDLE_ENTRIES {
            return Err(LaunchError::Identity);
        }
        names.push(entry.map_err(LaunchError::Io)?.file_name());
    }
    names.sort();
    Ok(names)
}
