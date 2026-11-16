//! Staged client-owned private-file reads for ADR 0097.
//!
//! [Decision Log]
//! - Purpose: bound bytes before UTF-8/TOML allocation and reject redirected
//!   or special book/credential targets before a blocking read.
//! - Existing asset: the workspace's safe rustix openat pattern. std's
//!   `read_to_string` alone cannot supply NOFOLLOW/NONBLOCK.
//! - Choice: one directory handle and one finite reader; errors carry only
//!   safe OS/UTF-8 causes, never the original bytes.
//! - Status: private Unix test staging only. No production book/credential
//!   caller, worker admission, lock, mutation or Windows API is wired yet.
//!   Do not present this as an available upsert or file-safety feature.

use std::collections::TryReserveError;
use std::io::{self, Read};
use std::path::{Component, Path};
use std::str::Utf8Error;

mod output;
mod unix;
use unix::Directory;

const MAX_BOOK_INPUT_BYTES: ByteLimit = ByteLimit(1_048_576);
const MAX_CREDENTIAL_FILE_BYTES: ByteLimit = ByteLimit(4_096);
const MAX_BOOK_PATH_BYTES: usize = 4_096;

/// Kept typed even inside the file boundary; these are bytes, not characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ByteLimit(u32);
impl std::fmt::Display for ByteLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Which independently bounded format its owner will decode afterwards.
#[derive(Debug, Clone, Copy)]
enum Purpose {
    Book,
    Credential,
}
impl Purpose {
    const fn limit(self) -> ByteLimit {
        match self {
            Self::Book => MAX_BOOK_INPUT_BYTES,
            Self::Credential => MAX_CREDENTIAL_FILE_BYTES,
        }
    }
}

/// One ordinary directory entry, never a path traversal or diagnostic echo.
#[derive(Debug, Clone, Copy)]
struct Name<'a>(&'a str);
impl<'a> Name<'a> {
    fn parse(value: &'a str) -> Result<Self, Error> {
        if value.len() > MAX_BOOK_PATH_BYTES
            || value.contains(['/', '\\'])
            || value.as_bytes().contains(&0)
        {
            return Err(Error::UnsafeName);
        }
        let mut components = Path::new(value).components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(_)), None) => Ok(Self(value)),
            _ => Err(Error::UnsafeName),
        }
    }
}

/// Opaque UTF-8 file text. Only the owning book/credential decoder may inspect it.
struct Text(String);
impl Text {
    fn as_str(&self) -> &str {
        &self.0
    }
}

/// A safe boundary fault; none stores bytes or a path supplied by file content.
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("the private file entry name is unsafe")]
    UnsafeName,
    #[error("the private file target is not an ordinary owned file")]
    UnsafeTarget,
    #[error("the private file exceeds {limit} bytes")]
    TooLarge { limit: ByteLimit },
    #[error("the private file byte limit does not fit this platform")]
    PlatformLimit,
    #[error("the private file byte limit does not fit this platform")]
    Capacity(#[source] std::num::TryFromIntError),
    #[error("private file allocation failed")]
    Allocation(#[source] TryReserveError),
    #[error("private file I/O failed")]
    Io(#[source] io::Error),
    #[error("private document formatting failed")]
    Format(#[source] std::fmt::Error),
    #[error("the private file is not UTF-8")]
    Utf8(#[source] Utf8Error),
}

/// The one-byte EOF probe stays on the stack, outside the capped document
/// buffer. Two maximum input/output pairs therefore keep ADR0097's exact
/// encoded-byte budget; this is not a claim about total allocator overhead.
fn read_limited(reader: &mut impl Read, limit: ByteLimit) -> Result<Text, Error> {
    let capacity = usize::try_from(limit.0).map_err(Error::Capacity)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(Error::Allocation)?;
    // A finite zeroed slice avoids Read::read_to_end growing a full Vec just
    // to ask for EOF. The file worker is rare and its maximum is one MiB.
    bytes.resize(capacity, 0);
    let mut filled = 0_usize;
    while filled < capacity {
        let remaining = bytes.get_mut(filled..).ok_or(Error::PlatformLimit)?;
        let read = read_retry(reader, remaining)?;
        if read == 0 {
            break;
        }
        filled = filled.checked_add(read).ok_or(Error::PlatformLimit)?;
    }
    bytes.truncate(filled);
    if filled == capacity && read_retry(reader, &mut [0_u8; 1])? != 0 {
        return Err(Error::TooLarge { limit });
    }
    String::from_utf8(bytes)
        .map(Text)
        // FromUtf8Error's Debug contains the original bytes. Keep only its
        // Utf8Error cause, which identifies the position/kind without them.
        .map_err(|error| Error::Utf8(error.utf8_error()))
}

/// Interrupted syscalls retain the admitted worker; no replacement task or
/// additional document allocation is made while retrying that same read.
fn read_retry(reader: &mut impl Read, bytes: &mut [u8]) -> Result<usize, Error> {
    loop {
        match reader.read(bytes) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            result => return result.map_err(Error::Io),
        }
    }
}

#[cfg(test)]
mod tests;
