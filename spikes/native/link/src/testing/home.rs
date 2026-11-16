//! One newly created temporary book directory, never the user's discovered book.
use super::{FixtureError, Phase};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

pub(crate) struct Home(pub(crate) PathBuf);
impl Home {
    pub(super) fn try_new() -> Result<Self, FixtureError> {
        let id = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_exhausted| FixtureError::Exhausted)?;
        let path =
            std::env::temp_dir().join(format!("vessel-machine-actor-{}-{id}", std::process::id()));
        create_private(&path).map_err(|source| FixtureError::Io {
            phase: Phase::Home,
            source,
        })?;
        Ok(Self(path))
    }
    pub(crate) fn book(&self) -> PathBuf {
        self.0.join("nodes.toml")
    }
    pub(super) fn remove(&self) -> Result<(), FixtureError> {
        match std::fs::remove_dir_all(&self.0) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(FixtureError::Io {
                phase: Phase::Home,
                source,
            }),
        }
    }
}
impl Drop for Home {
    fn drop(&mut self) {
        // Explicit scenario close reports cleanup errors. A failed/unwinding
        // test still attempts cleanup, without logging private directory text.
        let _removed = self.remove();
    }
}

fn create_private(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        std::fs::DirBuilder::new().mode(0o700).create(path)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir(path)
    }
}
