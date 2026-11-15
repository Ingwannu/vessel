//! The VAPID key is separate from the node's Noise identity. Losing it
//! invalidates browser subscriptions, never existing Vessel pairings.
//!
//! [Decision Log]
//! - Purpose: account-free signing whose private material never leaves the node.
//! - Choice: reuse atomic create-once and zeroized secret storage, but refuse
//!   links, non-regular files and non-private existing modes before reading.
//!   Bad existing keys are never regenerated silently.
//! - Cost: on Windows privacy still relies on the data directory's inherited
//!   account ACL, as the node's other credentials do; Unix is checked here.
use crate::{DataDir, NodeKeySecret, files};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read as _};
use std::path::PathBuf;
use zeroize::Zeroizing;

/// A VAPID private scalar, wiped on drop, with no printable secret form.
#[derive(Clone)]
pub struct PushPrivateKey(NodeKeySecret);
impl PushPrivateKey {
    /// Private signing bytes, only for the cryptographic boundary.
    #[must_use]
    pub fn expose(&self) -> &[u8; 32] {
        self.0.expose()
    }
}
impl fmt::Debug for PushPrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PushPrivateKey(<redacted>)")
    }
}
/// Why the persistent VAPID identity could not be used; no key is echoed.
#[derive(Debug, thiserror::Error)]
pub enum PushKeyError {
    /// Cannot safely open or create the key.
    #[error("cannot use the private push key at {}", path.display())]
    Io {
        /// The key's path, never its contents.
        path: PathBuf,
        /// Operating system failure.
        #[source]
        source: io::Error,
    },
    /// Incorrect mode, owner or file kind.
    #[error(
        "push-vapid.key must be an owner-only regular file; restore its ownership and mode before starting the node"
    )]
    NotPrivate,
    /// An existing key is corrupt and must be restored intentionally.
    #[error(
        "push-vapid.key must contain exactly 32 bytes; restore the key or remove it and subscribe devices again"
    )]
    InvalidSize,
    /// Secure randomness is unavailable.
    #[error("cannot generate a private push key")]
    Random(#[source] getrandom::Error),
}
impl DataDir {
    /// Load the private VAPID scalar or atomically create it once.
    ///
    /// # Errors
    /// Existing links, files with unsafe ownership/mode, corrupt length,
    /// I/O failure, or unavailable system randomness.
    pub fn load_or_create_push_key(&self) -> Result<PushPrivateKey, PushKeyError> {
        let path = self.root().join("push-vapid.key");
        match read(self, &path) {
            Ok(key) => return Ok(key),
            Err(PushKeyError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let fresh = NodeKeySecret::generate()
            .map(PushPrivateKey)
            .map_err(PushKeyError::Random)?;
        let created =
            files::create_once(&path, fresh.expose()).map_err(|source| PushKeyError::Io {
                path: path.clone(),
                source,
            })?;
        if created {
            Ok(fresh)
        } else {
            read(self, &path)
        }
    }
}
fn read(dir: &DataDir, path: &std::path::Path) -> Result<PushPrivateKey, PushKeyError> {
    let failed = |source| PushKeyError::Io {
        path: path.to_owned(),
        source,
    };
    let mut file = open(path).map_err(failed)?;
    let metadata = file.metadata().map_err(failed)?;
    if !metadata.is_file() {
        return Err(PushKeyError::NotPrivate);
    }
    check_private(dir, &metadata)?;
    if metadata.len() != 32 {
        return Err(PushKeyError::InvalidSize);
    }
    let mut bytes = Zeroizing::new([0_u8; 32]);
    file.read_exact(bytes.as_mut_slice()).map_err(failed)?;
    let mut extra = [0_u8; 1];
    if file.read(&mut extra).map_err(failed)? != 0 {
        return Err(PushKeyError::InvalidSize);
    }
    NodeKeySecret::from_stored(bytes.as_slice())
        .map(PushPrivateKey)
        .ok_or(PushKeyError::InvalidSize)
}
#[cfg(unix)]
fn open(path: &std::path::Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let flags = i32::try_from(rustix::fs::OFlags::NOFOLLOW.bits()).map_err(io::Error::other)?;
    OpenOptions::new().read(true).custom_flags(flags).open(path)
}
#[cfg(not(unix))]
fn open(path: &std::path::Path) -> io::Result<File> {
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "push key cannot be a link",
        ));
    }
    OpenOptions::new().read(true).open(path)
}
#[cfg(unix)]
fn check_private(dir: &DataDir, metadata: &std::fs::Metadata) -> Result<(), PushKeyError> {
    use std::os::unix::fs::MetadataExt as _;
    let parent = std::fs::metadata(dir.root()).map_err(|source| PushKeyError::Io {
        path: dir.root().to_owned(),
        source,
    })?;
    if metadata.uid() != parent.uid()
        || metadata.mode() & 0o077 != 0
        || metadata.mode() & 0o400 == 0
    {
        return Err(PushKeyError::NotPrivate);
    }
    Ok(())
}
#[cfg(not(unix))]
fn check_private(_dir: &DataDir, _metadata: &std::fs::Metadata) -> Result<(), PushKeyError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrupt_existing_key_is_never_silently_replaced() {
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path());
        dir.prepare().unwrap();
        let path = temp.path().join("push-vapid.key");
        assert!(files::create_once(&path, &[7; 31]).unwrap());
        assert!(matches!(
            dir.load_or_create_push_key(),
            Err(PushKeyError::InvalidSize)
        ));
        assert_eq!(std::fs::read(path).unwrap(), [7; 31]);
    }
    #[cfg(unix)]
    #[test]
    fn unsafe_mode_and_symlink_are_refused_without_reading_or_replacing() {
        use std::os::unix::fs::{PermissionsExt as _, symlink};
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path());
        dir.prepare().unwrap();
        let path = temp.path().join("push-vapid.key");
        assert!(files::create_once(&path, &[7; 32]).unwrap());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            dir.load_or_create_push_key(),
            Err(PushKeyError::NotPrivate)
        ));
        std::fs::remove_file(&path).unwrap();
        let target = temp.path().join("other");
        assert!(files::create_once(&target, &[8; 32]).unwrap());
        symlink(&target, &path).unwrap();
        assert!(matches!(
            dir.load_or_create_push_key(),
            Err(PushKeyError::Io { .. })
        ));
        assert_eq!(std::fs::read(target).unwrap(), [8; 32]);
    }
}
