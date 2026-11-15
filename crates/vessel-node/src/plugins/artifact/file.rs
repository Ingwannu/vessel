//! A private regular-file probe, not a complete candidate-tree observation.
//!
//! Why (ADR0079): a pathname check alone can be replaced with a FIFO before
//! open. Nonblocking/no-follow open and descriptor validation must precede read.
//! Store's attachment reader has this policy for its own stored IDs; it cannot
//! observe arbitrary package files or confer candidate ownership here.
use super::{Limit, MAX_FILE_BYTES, Problem};
use sha2::{Digest as _, Sha256};
use std::{
    error::Error,
    fmt,
    fs::{self, File, Metadata, OpenOptions},
    io::{self, Read as _},
    os::unix::fs::{MetadataExt as _, OpenOptionsExt as _},
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::workspace::ContentHash;

const OBSERVATION_BUDGET: Duration = Duration::from_secs(30);
const READ_CHUNK_BYTES: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
enum Failure {
    #[error("private plugin file I/O failed")]
    Io(#[source] PrivateIo),
    #[error("private plugin file policy refused the observation")]
    Policy,
    #[error("private plugin file changed during observation")]
    Changed,
    #[error("private plugin file exceeds its observation budget")]
    Capacity(#[source] Problem),
    #[error("private plugin file observation was cancelled")]
    Cancelled,
    #[error("private plugin file observation expired")]
    Expired,
}

/// Retain the concrete cause, but never publish its input-bearing text/chain.
struct PrivateIo(io::Error);
impl fmt::Debug for PrivateIo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrivateIo")
            .field("kind", &self.0.kind())
            .field("os_code", &self.0.raw_os_error())
            .finish()
    }
}
impl fmt::Display for PrivateIo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("private plugin file I/O error")
    }
}
impl Error for PrivateIo {}
fn io_failure(cause: io::Error) -> Failure {
    Failure::Io(PrivateIo(cause))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    bytes: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl From<&Metadata> for Stamp {
    fn from(m: &Metadata) -> Self {
        Self {
            device: m.dev(),
            inode: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            bytes: m.len(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
        }
    }
}

/// A bounded synchronous probe. A future blocking owner must retain its permit
/// through settlement; these between-read checks cannot interrupt a syscall.
struct Probe {
    path: PathBuf,
    stamp: Stamp,
}
impl Probe {
    fn prepare(path: PathBuf, device: u64) -> Result<Self, Failure> {
        let metadata = fs::symlink_metadata(&path).map_err(io_failure)?;
        regular(&metadata, device)?;
        Ok(Self {
            path,
            stamp: Stamp::from(&metadata),
        })
    }

    fn read(self, cancellation: &CancellationToken) -> Result<ContentHash, Failure> {
        self.read_until(cancellation, Instant::now() + OBSERVATION_BUDGET)
    }

    fn read_until(
        self,
        cancellation: &CancellationToken,
        deadline: Instant,
    ) -> Result<ContentHash, Failure> {
        check(cancellation, deadline)?;
        let mut file = self.open()?;
        let mut hash = Sha256::new();
        let mut buffer = [0_u8; READ_CHUNK_BYTES];
        let mut count = 0_u64;
        loop {
            check(cancellation, deadline)?;
            let read = file.read(&mut buffer).map_err(io_failure)?;
            if read == 0 {
                break;
            }
            count = count.saturating_add(
                u64::try_from(read).map_err(|cause| Failure::Capacity(Problem::Length(cause)))?,
            );
            if count > self.stamp.bytes {
                return Err(Failure::Changed);
            }
            hash.update(&buffer[..read]);
        }
        check(cancellation, deadline)?;
        self.verify(&file)?;
        let after = fs::symlink_metadata(&self.path).map_err(io_failure)?;
        if count != self.stamp.bytes || Stamp::from(&after) != self.stamp {
            return Err(Failure::Changed);
        }
        Ok(ContentHash::from_sha256(hash.finalize().into()))
    }

    fn verify(&self, file: &File) -> Result<(), Failure> {
        let metadata = file.metadata().map_err(io_failure)?;
        regular(&metadata, self.stamp.device)?;
        if Stamp::from(&metadata) != self.stamp {
            return Err(Failure::Changed);
        }
        Ok(())
    }

    fn open(&self) -> Result<File, Failure> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_NONBLOCK | nix::libc::O_NOFOLLOW)
            .open(&self.path)
            .map_err(io_failure)?;
        self.verify(&file)?;
        Ok(file)
    }
}

fn regular(metadata: &Metadata, device: u64) -> Result<(), Failure> {
    // Refuse a special file before a potentially blocking open, then repeat on
    // the actual opened descriptor. Device equality is not bind-mount proof.
    if !metadata.is_file()
        || metadata.uid() != nix::unistd::geteuid().as_raw()
        || metadata.mode() & 0o022 != 0
        || metadata.dev() != device
    {
        return Err(Failure::Policy);
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(Failure::Capacity(Problem::Capacity(Limit::FileBytes)));
    }
    Ok(())
}

fn check(cancellation: &CancellationToken, deadline: Instant) -> Result<(), Failure> {
    if cancellation.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(Failure::Expired);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
