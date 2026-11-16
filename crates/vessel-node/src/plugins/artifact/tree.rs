//! Complete synchronous observations, still private under plugin staging.
//!
//! Why (ADR0079): a declaration hash or a supplied record list cannot establish
//! what executable/assets were actually observed. Neither can this grant source
//! provenance or execution authority, and it does not own an async admission lane.
use super::{
    Budget, Kind, LinkTarget, PackagePath, Problem, Record, TreeDigest, file, package_path,
};
use std::{
    error::Error,
    ffi::OsString,
    fmt, fs,
    os::unix::fs::MetadataExt as _,
    path::{Path, PathBuf},
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use vessel_workspace::paths::{PathError, Root};

mod walk;
use walk::Pass;

#[derive(Debug, thiserror::Error)]
pub(super) enum Failure {
    #[error("private plugin tree file observation failed")]
    File(#[from] file::Failure),
    #[error("private plugin tree name or resource budget refused observation")]
    Shape(#[from] Problem),
    #[error("private plugin tree root could not be resolved")]
    Root(#[source] PrivateRoot),
    #[error("private plugin tree name is not representable")]
    Name(#[source] PrivateName),
    #[error("private plugin tree policy refused observation")]
    Policy,
    #[error("private plugin tree changed during observation")]
    Changed,
}

pub(super) struct PrivateRoot(PathError);
impl fmt::Debug for PrivateRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match &self.0 {
            PathError::Io(cause) => {
                return f.debug_tuple("PrivateRoot").field(&cause.kind()).finish();
            }
            PathError::Outside(_) => "outside",
            PathError::GitDir(_) => "git",
            PathError::NotFound(_) => "missing",
            PathError::Root => "root",
            PathError::Platform(_) => "platform",
        };
        f.debug_tuple("PrivateRoot").field(&category).finish()
    }
}
impl fmt::Display for PrivateRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("private plugin root is unavailable")
    }
}
impl Error for PrivateRoot {}

pub(super) struct PrivateName(OsString);
impl fmt::Debug for PrivateName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrivateName")
            .field("bytes", &self.0.len())
            .finish()
    }
}
impl fmt::Display for PrivateName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("private plugin name is not representable")
    }
}
impl Error for PrivateName {}

struct Scope {
    root: Root,
    stamp: file::Stamp,
    device: u64,
    deadline: Instant,
    cancellation: CancellationToken,
}
impl Scope {
    fn new(path: &Path, cancellation: &CancellationToken) -> Result<Self, Failure> {
        let deadline = Instant::now()
            .checked_add(file::OBSERVATION_BUDGET)
            .ok_or(file::Failure::Expired)?;
        Self::new_until(path, cancellation, deadline)
    }

    fn new_until(
        path: &Path,
        cancellation: &CancellationToken,
        deadline: Instant,
    ) -> Result<Self, Failure> {
        file::check(cancellation, deadline)?;
        let before = fs::symlink_metadata(path).map_err(file::io_failure)?;
        directory(&before, before.dev())?;
        let root = Root::open(path).map_err(|cause| Failure::Root(PrivateRoot(cause)))?;
        let scope = Self {
            root,
            stamp: file::Stamp::from(&before),
            device: before.dev(),
            deadline,
            cancellation: cancellation.clone(),
        };
        scope.check_root()?;
        Ok(scope)
    }

    fn check(&self) -> Result<(), Failure> {
        Ok(file::check(&self.cancellation, self.deadline)?)
    }

    fn check_root(&self) -> Result<(), Failure> {
        self.check()?;
        let after = fs::symlink_metadata(self.root.path()).map_err(file::io_failure)?;
        directory(&after, self.device)?;
        if file::Stamp::from(&after) != self.stamp {
            return Err(Failure::Changed);
        }
        Ok(())
    }
}

pub(super) fn directory(metadata: &fs::Metadata, device: u64) -> Result<(), Failure> {
    if !metadata.is_dir()
        || metadata.uid() != nix::unistd::geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
        || metadata.dev() != device
    {
        return Err(Failure::Policy);
    }
    Ok(())
}

/// First-pass state is deliberately not a successful artifact identity.
pub(super) struct Pending {
    scope: Scope,
    first: Pass,
}

/// Only both actual passes can construct this observation. The supplied-record
/// digest remains a different type, and neither is managed source authority.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Observed {
    digest: TreeDigest,
}
impl Pending {
    pub(super) fn begin(path: &Path, cancellation: &CancellationToken) -> Result<Self, Failure> {
        let scope = Scope::new(path, cancellation)?;
        Self::from_scope(scope)
    }

    pub(super) fn begin_until(
        path: &Path,
        cancellation: &CancellationToken,
        deadline: Instant,
    ) -> Result<Self, Failure> {
        Self::from_scope(Scope::new_until(path, cancellation, deadline)?)
    }

    fn from_scope(scope: Scope) -> Result<Self, Failure> {
        let first = Pass::read(&scope)?;
        Ok(Self { scope, first })
    }

    pub(super) fn finish(self) -> Result<Observed, Failure> {
        let second = Pass::read(&self.scope)?;
        if self.first != second {
            return Err(Failure::Changed);
        }
        self.scope.check_root()?;
        Ok(Observed {
            digest: second.digest,
        })
    }
}

#[cfg(test)]
mod tests;
