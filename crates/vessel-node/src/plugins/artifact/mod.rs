//! Supplied-record facts and staged Unix tree observations (ADR0079).
//! `TreeDigest` alone does not prove complete I/O; the private tree owner performs
//! two real passes. Neither grants source provenance or execution authority.
//! A private tracked blocking adapter retains admitted work; actual host
//! admission, candidate acquisition and non-Unix ACL qualification remain absent.
#[cfg(target_os = "linux")]
mod directory;
#[cfg(unix)]
mod file;
#[cfg(unix)]
mod operation;
#[cfg(test)]
mod tests;
#[cfg(unix)]
mod tree;

use super::manifest::paths::{InvalidPackagePath, PackagePath};
use sha2::{Digest as _, Sha256};
use std::{error::Error, fmt};
use vessel_proto::workspace::ContentHash;

const MAX_ENTRIES: usize = 32 * 1024;
const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TOTAL_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_NAME_BYTES: usize = 4096;
const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Limit {
    Entries,
    FileBytes,
    TotalBytes,
    NameBytes,
    Depth,
}
#[derive(Debug, thiserror::Error)]
enum Problem {
    #[error("plugin candidate observation exceeds its resource budget ({0:?})")]
    Capacity(Limit),
    #[error("plugin candidate entry name is invalid")]
    Name(#[source] PrivatePath),
    #[error("plugin candidate contains a duplicate entry name")]
    Duplicate,
    #[error("plugin candidate link spelling is invalid")]
    Link,
    #[error("plugin candidate length cannot be represented")]
    Length(#[source] std::num::TryFromIntError),
}

/// The concrete existing path parser stays sealed across every diagnostic path.
struct PrivatePath(InvalidPackagePath);
impl fmt::Debug for PrivatePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Read the classification without formatting its input-bearing cause.
        f.write_str(match &self.0 {
            InvalidPackagePath::Path(_) => "PrivatePath(path)",
            InvalidPackagePath::Policy => "PrivatePath(policy)",
            InvalidPackagePath::Url(_) => "PrivatePath(url)",
        })
    }
}
impl fmt::Display for PrivatePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid private candidate path")
    }
}
impl Error for PrivatePath {}

/// Only a bounded spelling; the future reader must independently check that its
/// actual resolution stays inside the owned candidate. Never a path capability.
struct LinkTarget(String);
impl LinkTarget {
    fn new(raw: &str) -> Result<Self, Problem> {
        if raw.len() > MAX_NAME_BYTES {
            return Err(Problem::Capacity(Limit::NameBytes));
        }
        if raw.is_empty() || raw.contains('\0') {
            return Err(Problem::Link);
        }
        Ok(Self(raw.to_owned()))
    }
}

enum Kind {
    Directory {
        executable: bool,
    },
    File {
        bytes: u64,
        executable: bool,
        hash: ContentHash,
    },
    Link {
        target: LinkTarget,
    },
}
struct Record {
    path: PackagePath,
    kind: Kind,
}
impl Record {
    fn new(raw: &str, kind: Kind) -> Result<Self, Problem> {
        Ok(Self {
            path: package_path(raw)?,
            kind,
        })
    }
}
fn package_path(raw: &str) -> Result<PackagePath, Problem> {
    if raw.len() > MAX_NAME_BYTES {
        return Err(Problem::Capacity(Limit::NameBytes));
    }
    PackagePath::try_from(raw.to_owned()).map_err(|cause| Problem::Name(PrivatePath(cause)))
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Budget {
    entries: usize,
    file_bytes: u64,
}
impl Budget {
    /// Refusal leaves every previously admitted counter unchanged.
    fn admit(&mut self, record: &Record) -> Result<(), Problem> {
        let bytes = match record.kind {
            Kind::File { bytes, .. } => bytes,
            Kind::Directory { .. } | Kind::Link { .. } => 0,
        };
        self.admit_shape(&record.path, bytes)
    }

    fn admit_shape(&mut self, path: &PackagePath, bytes: u64) -> Result<(), Problem> {
        if path.as_str().split('/').count() > MAX_DEPTH {
            return Err(Problem::Capacity(Limit::Depth));
        }
        let entries = self.entries.saturating_add(1);
        if entries > MAX_ENTRIES {
            return Err(Problem::Capacity(Limit::Entries));
        }
        if bytes > MAX_FILE_BYTES {
            return Err(Problem::Capacity(Limit::FileBytes));
        }
        let file_bytes = self.file_bytes.saturating_add(bytes);
        if file_bytes > MAX_TOTAL_FILE_BYTES {
            return Err(Problem::Capacity(Limit::TotalBytes));
        }
        *self = Self {
            entries,
            file_bytes,
        };
        Ok(())
    }
}

/// A deterministic digest of SUPPLIED facts, not a stable filesystem receipt.
#[derive(PartialEq, Eq)]
struct TreeDigest {
    digest: ContentHash,
    budget: Budget,
}
impl fmt::Debug for TreeDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeDigest")
            .field("budget", &self.budget)
            .finish_non_exhaustive()
    }
}
impl TreeDigest {
    fn new(mut records: Vec<Record>) -> Result<Self, Problem> {
        let mut budget = Budget::default();
        for record in &records {
            budget.admit(record)?;
        }
        records.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));
        if records
            .windows(2)
            .any(|pair| matches!(pair, [left, right] if left.path.as_str() == right.path.as_str()))
        {
            return Err(Problem::Duplicate);
        }
        let mut hash = Sha256::new();
        hash.update(b"vessel-plugin-tree-v1\0");
        for record in records {
            field(&mut hash, record.path.as_str().as_bytes())?;
            match record.kind {
                Kind::Directory { executable } => hash.update([0, u8::from(executable)]),
                Kind::File {
                    bytes,
                    executable,
                    hash: content,
                } => {
                    hash.update([1, u8::from(executable)]);
                    hash.update(bytes.to_le_bytes());
                    field(&mut hash, content.as_str().as_bytes())?;
                }
                Kind::Link { target } => {
                    hash.update([2]);
                    field(&mut hash, target.0.as_bytes())?;
                }
            }
        }
        Ok(Self {
            digest: ContentHash::from_sha256(hash.finalize().into()),
            budget,
        })
    }
}
fn field(hash: &mut Sha256, bytes: &[u8]) -> Result<(), Problem> {
    hash.update(
        u64::try_from(bytes.len())
            .map_err(Problem::Length)?
            .to_le_bytes(),
    );
    hash.update(bytes);
    Ok(())
}
