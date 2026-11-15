//! Read-only runtime identity, without an installation path or qualification grant.
use sha2::{Digest as _, Sha256};
use std::fs::{self, File};
use std::os::unix::fs::MetadataExt as _;
use std::path::Path;
use vessel_proc::Child;

const BROWSER: &str = "/opt/vessel-cft/chromium-1228/chrome-linux64/chrome";
const FILE_READ_BYTES: usize = 64 * 1024;
/// Bound corrupted/replaced candidate files before streaming their contents.
const MAX_EXECUTABLE_BYTES: u64 = 512 * 1024 * 1024;
const EXECUTE_ACCESS: u32 = 0o001;
const WRITE_ACCESS: u32 = 0o022;
const ROOT_UID: u32 = 0;
// These identify only the diagnostic candidate. No complete qualification exists.
const BROWSER_DIGEST: [u8; 32] = [
    45, 24, 219, 157, 134, 8, 176, 82, 182, 165, 82, 238, 0, 236, 30, 131, 15, 147, 105, 46, 146,
    139, 101, 236, 198, 125, 105, 59, 211, 63, 232, 1,
];
const SHELL_DIGEST: [u8; 32] = [
    134, 211, 31, 111, 183, 153, 233, 31, 162, 27, 173, 52, 20, 132, 86, 69, 16, 202, 40, 119, 3,
    161, 110, 158, 70, 197, 51, 56, 119, 111, 79, 66,
];

#[derive(Debug, thiserror::Error)]
pub(crate) enum LaunchError {
    #[error("browser runtime identity could not be read")]
    Io(#[source] std::io::Error),
    #[error("browser runtime or launcher identity differs from its candidate")]
    Identity,
    #[error("browser runtime executable must be root-owned and immutable to other accounts")]
    Permissions,
    #[error("browser policy proxy must use its private IPv4 loopback listener")]
    Proxy,
    #[error("browser execution requires a non-root service account")]
    Account,
}
/// Hashes alone never become a `QualifiedRuntime` token.
pub(super) struct Candidate;
impl Candidate {
    pub(super) fn inspect() -> Result<Self, LaunchError> {
        if nix::unistd::geteuid().is_root() {
            return Err(LaunchError::Account);
        }
        verify(Path::new(BROWSER), &BROWSER_DIGEST)?;
        verify(Path::new(super::FIXED_SHELL), &SHELL_DIGEST)?;
        Ok(Self)
    }
    pub(super) const fn browser_path(&self) -> &'static str {
        BROWSER
    }
    pub(super) fn verify_started(&self, child: &Child) -> Result<(), LaunchError> {
        if !child.lifetime().is_live() {
            return Err(LaunchError::Identity);
        }
        // Borrowing the actual Child excludes this owner reaping it concurrently.
        // The debugger pipes must also belong to this same supervised child.
        let executable = format!("/proc/{}/exe", child.pid());
        verify(Path::new(&executable), &BROWSER_DIGEST)
    }
}
fn verify(path: &Path, expected: &[u8; 32]) -> Result<(), LaunchError> {
    let actual = fs::canonicalize(path).map_err(LaunchError::Io)?;
    // All executable parents must be root-owned and not writable by another
    // account; root/operator replacement is caught by hashes again after spawn.
    for parent in actual.ancestors().skip(1) {
        let metadata = fs::metadata(parent).map_err(LaunchError::Io)?;
        if metadata.uid() != ROOT_UID || metadata.mode() & WRITE_ACCESS != 0 {
            return Err(LaunchError::Permissions);
        }
    }
    let initial = fs::metadata(path).map_err(LaunchError::Io)?;
    if !initial.is_file() || initial.len() > MAX_EXECUTABLE_BYTES {
        return Err(LaunchError::Identity);
    }
    let mut file = File::open(path).map_err(LaunchError::Io)?;
    let metadata = file.metadata().map_err(LaunchError::Io)?;
    if !metadata.is_file() || metadata.uid() != ROOT_UID || metadata.mode() & WRITE_ACCESS != 0 {
        return Err(LaunchError::Permissions);
    }
    if digest(&mut file)? != *expected {
        return Err(LaunchError::Identity);
    }
    Ok(())
}
fn digest(reader: &mut impl std::io::Read) -> Result<[u8; 32], LaunchError> {
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; FILE_READ_BYTES];
    let mut remaining = MAX_EXECUTABLE_BYTES;
    loop {
        let count = reader.read(&mut buffer).map_err(LaunchError::Io)?;
        if count == 0 {
            break;
        }
        remaining = remaining
            .checked_sub(u64::try_from(count).map_err(|_overflow| LaunchError::Identity)?)
            .ok_or(LaunchError::Identity)?;
        hash.update(buffer.get(..count).ok_or(LaunchError::Identity)?);
    }
    Ok(hash.finalize().into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hashing_is_streamed_and_a_candidate_cannot_accept_a_replaced_program() {
        let mut bytes = &b"abc"[..];
        assert_eq!(
            digest(&mut bytes).unwrap(),
            [
                186, 120, 22, 191, 143, 1, 207, 234, 65, 65, 64, 222, 93, 174, 34, 35, 176, 3, 97,
                163, 150, 23, 122, 156, 180, 16, 255, 97, 242, 0, 21, 173
            ]
        );
        // A root-owned system binary is not accepted merely because it has
        // the same path as a launcher; a different digest always refuses.
        assert!(matches!(
            verify(Path::new(super::super::FIXED_SHELL), &BROWSER_DIGEST),
            Err(LaunchError::Identity)
        ));
    }
}
