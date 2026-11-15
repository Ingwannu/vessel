//! Length-framed names and fixed field units prevent manifest concatenation aliases.
use super::{Fingerprint, LaunchError};
use sha2::{Digest as _, Sha256};
use std::path::Path;

enum Kind {
    Directory,
    File,
}
impl Kind {
    const fn tag(self) -> u8 {
        match self {
            Self::Directory => b'D',
            Self::File => b'F',
        }
    }
}
pub(super) struct ManifestHash(Sha256);
impl ManifestHash {
    pub(super) fn new() -> Self {
        let mut hash = Sha256::new();
        hash.update(b"vessel-candidate-bundle-v1");
        Self(hash)
    }
    fn entry(&mut self, kind: Kind, path: &Path, mode: u32) -> Result<(), LaunchError> {
        let name = path.as_os_str().as_encoded_bytes();
        self.0.update([kind.tag()]);
        self.0.update(
            u64::try_from(name.len())
                .map_err(|_overflow| LaunchError::Identity)?
                .to_le_bytes(),
        );
        self.0.update(name);
        self.0.update(mode.to_le_bytes());
        Ok(())
    }
    pub(super) fn directory(&mut self, path: &Path, mode: u32) -> Result<(), LaunchError> {
        self.entry(Kind::Directory, path, mode)
    }
    pub(super) fn file(
        &mut self,
        path: &Path,
        mode: u32,
        length: u64,
        digest: [u8; 32],
    ) -> Result<(), LaunchError> {
        self.entry(Kind::File, path, mode)?;
        self.0.update(length.to_le_bytes());
        self.0.update(digest);
        Ok(())
    }
    pub(super) fn finish(self) -> Fingerprint {
        Fingerprint(self.0.finalize().into())
    }
}
