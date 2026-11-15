//! Private immutable byte snapshots, with one barrier for writes and GC.
//! Active runtime leases and a validated manifest both protect a managed file.
use chrono::Utc;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};
use vessel_app_view::composer::local::{LocalFile, LocalKey, LocalProblem, LocalRecord, LocalSize};
use vessel_proto::{AttachmentName, conversation::MimeType};

mod manifest;
pub use manifest::{KeptFile, Manifest};

/// A cache failure never silently discards a selected attachment.
#[derive(Debug, thiserror::Error)]
pub enum LocalFailure {
    /// File operations retain their original I/O cause.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Metadata must pass the same validator on load and on intake.
    #[error(transparent)]
    Invalid(#[from] LocalProblem),
    /// No app state directory is available.
    #[error("local attachment storage is unavailable")]
    Unavailable,
    /// A managed root/file was replaced with a symlink or shared-access entry.
    #[error("the local attachment storage boundary changed")]
    Boundary,
    /// The source changed while copying; keep no incomplete managed snapshot.
    #[error("the attached file changed while being copied")]
    Changed,
    /// A poisoned barrier is not permission to run GC concurrently.
    #[error("the local attachment storage barrier is unavailable")]
    Barrier,
    /// The bounded managed store has no room for another immutable snapshot.
    #[error("local attachment storage is full")]
    Capacity,
    /// Unknown/corrupt state is preserved rather than overwritten or collected.
    #[error("the local attachment manifest is invalid")]
    Manifest,
    /// Unique local keys may not wrap or overwrite another file.
    #[error("local attachment keys exhausted")]
    Exhausted,
}

#[derive(Debug)]
struct Inner {
    root: PathBuf,
    leases: BTreeMap<LocalKey, Weak<LocalFile>>,
}
/// Clones share one write/collection barrier, never separate competing GCs.
#[derive(Debug, Clone)]
pub struct Store(Arc<Mutex<Inner>>);

/// Newly saved/unreferenced bytes survive a finalized-draft grace interval.
const GRACE: Duration = Duration::from_mins(5);
const KEY_ATTEMPTS: usize = 8;
/// Disk usage is bounded even while finalized files await grace-period GC.
const MAX_TOTAL: u64 = 512 * 1024 * 1024;
const MAX_FILES: usize = 512;
static NEXT: AtomicU64 = AtomicU64::new(0);

impl Store {
    /// Resolve only the app's existing private state location.
    ///
    /// # Errors
    /// Returns a typed storage failure without changing user configuration.
    pub fn open() -> Result<Self, LocalFailure> {
        let path = crate::offers::Store::default_path()
            .and_then(|path| path.parent().map(|parent| parent.join("attachment-bytes")))
            .ok_or(LocalFailure::Unavailable)?;
        Self::at(&path)
    }
    /// Explicit roots allow isolated tests without modifying process environment.
    ///
    /// # Errors
    /// Refuses shared permissions, symlinks and non-directory roots.
    pub fn at(root: &Path) -> Result<Self, LocalFailure> {
        crate::prefs::private_dir(root)?;
        private_entry(root, true)?;
        Ok(Self(Arc::new(Mutex::new(Inner {
            root: root.to_owned(),
            leases: BTreeMap::new(),
        }))))
    }
    /// Persist bytes before adding a draft pill, never on the UI executor.
    ///
    /// # Errors
    /// Refuses invalid lengths or incomplete writes and retains their cause.
    pub fn put_bytes(
        &self,
        name: AttachmentName,
        mime: MimeType,
        bytes: &[u8],
    ) -> Result<Arc<LocalFile>, LocalFailure> {
        let size =
            LocalSize::new(u64::try_from(bytes.len()).map_err(|_overflow| LocalProblem::Size)?)?;
        self.put(name, mime, size, |out| {
            out.write_all(bytes)?;
            Ok(())
        })
    }
    /// Snapshot a picked file; later edits to the source cannot alter the send.
    ///
    /// # Errors
    /// Refuses a non-file, oversized input or a source changed during copying.
    pub fn copy_file(
        &self,
        name: AttachmentName,
        mime: MimeType,
        source: &Path,
    ) -> Result<Arc<LocalFile>, LocalFailure> {
        let mut input = std::fs::File::open(source)?;
        let before = input.metadata()?;
        if !before.is_file() {
            return Err(LocalFailure::Boundary);
        }
        let size = LocalSize::new(before.len())?;
        self.put(name, mime, size, |out| {
            let copied =
                std::io::copy(&mut (&mut input).take(size.value().saturating_add(1)), out)?;
            let after = input.metadata()?;
            if copied != size.value()
                || before.len() != after.len()
                || before.modified()? != after.modified()?
            {
                return Err(LocalFailure::Changed);
            }
            Ok(())
        })
    }
    fn put(
        &self,
        name: AttachmentName,
        mime: MimeType,
        size: LocalSize,
        write: impl FnOnce(&mut std::fs::File) -> Result<(), LocalFailure>,
    ) -> Result<Arc<LocalFile>, LocalFailure> {
        let mut inner = self.0.lock().map_err(|_poisoned| LocalFailure::Barrier)?;
        private_entry(&inner.root, true)?;
        let mut total = size.value();
        let mut count = 0usize;
        for entry in std::fs::read_dir(&inner.root)? {
            let entry = entry?;
            let metadata = std::fs::symlink_metadata(entry.path())?;
            if metadata.is_file() {
                total = total
                    .checked_add(metadata.len())
                    .ok_or(LocalFailure::Capacity)?;
                count = count.saturating_add(1);
            }
        }
        if total > MAX_TOTAL || count >= MAX_FILES {
            return Err(LocalFailure::Capacity);
        }
        let (key, path, mut output) = create(&inner.root)?;
        let result = write(&mut output).and_then(|()| {
            output.sync_all()?;
            if output.metadata()?.len() != size.value() {
                return Err(LocalFailure::Changed);
            }
            Ok(())
        });
        drop(output);
        if let Err(problem) = result {
            std::fs::remove_file(&path)?;
            return Err(problem);
        }
        let local = Arc::new(LocalFile {
            path,
            record: LocalRecord {
                key: key.clone(),
                name,
                mime,
                size,
                created_at: Utc::now(),
            },
        });
        let _previous = inner.leases.insert(key, Arc::downgrade(&local));
        Ok(local)
    }
    /// Open only a validated managed key and verify its immutable length.
    ///
    /// # Errors
    /// Refuses missing, changed or substituted files.
    pub fn get(&self, record: &LocalRecord) -> Result<Arc<LocalFile>, LocalFailure> {
        let mut inner = self.0.lock().map_err(|_poisoned| LocalFailure::Barrier)?;
        private_entry(&inner.root, true)?;
        let path = file_path(&inner.root, &record.key);
        private_entry(&path, false)?;
        if std::fs::metadata(&path)?.len() != record.size.value() {
            return Err(LocalFailure::Changed);
        }
        if let Some(held) = inner.leases.get(&record.key).and_then(Weak::upgrade) {
            if held.record != *record {
                return Err(LocalFailure::Changed);
            }
            return Ok(held);
        }
        let local = Arc::new(LocalFile {
            record: record.clone(),
            path,
        });
        let _previous = inner
            .leases
            .insert(record.key.clone(), Arc::downgrade(&local));
        Ok(local)
    }
    /// GC accepts only a complete validated retained-set snapshot. A failed
    /// manifest load must be handled before calling this method.
    ///
    /// # Errors
    /// A directory/I/O/barrier failure aborts collection, preserving the cause.
    pub fn collect(&self, retained: &Manifest) -> Result<usize, LocalFailure> {
        self.collect_at(retained, SystemTime::now())
    }
    fn collect_at(&self, retained: &Manifest, now: SystemTime) -> Result<usize, LocalFailure> {
        let mut inner = self.0.lock().map_err(|_poisoned| LocalFailure::Barrier)?;
        private_entry(&inner.root, true)?;
        let persisted = manifest::read(&inner.root)?;
        let mut protected: BTreeSet<_> = retained
            .files()
            .iter()
            .map(|file| file.record.key.clone())
            .collect();
        protected.extend(persisted.files().iter().map(|file| file.record.key.clone()));
        inner.leases.retain(|key, weak| {
            let alive = weak.strong_count() > 0;
            if alive {
                let _new = protected.insert(key.clone());
            }
            alive
        });
        let mut removed = 0usize;
        for entry in std::fs::read_dir(&inner.root)? {
            let entry = entry?;
            let filename = entry.file_name();
            let Some(stem) = filename.to_str().and_then(|name| name.strip_suffix(".bin")) else {
                continue;
            };
            let Ok(key) = LocalKey::parse(stem) else {
                continue;
            };
            if protected.contains(&key) {
                continue;
            }
            let metadata = std::fs::symlink_metadata(entry.path())?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                continue;
            }
            if now
                .duration_since(metadata.modified()?)
                .is_ok_and(|age| age >= GRACE)
            {
                std::fs::remove_file(entry.path())?;
                removed = removed.saturating_add(1);
            }
        }
        Ok(removed)
    }
}

fn file_path(root: &Path, key: &LocalKey) -> PathBuf {
    root.join(format!("{}.bin", key.as_str()))
}
fn create(root: &Path) -> Result<(LocalKey, PathBuf, std::fs::File), LocalFailure> {
    for _attempt in 0..KEY_ATTEMPTS {
        let serial = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_exhausted| LocalFailure::Exhausted)?;
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_before_epoch| LocalFailure::Exhausted)?
            .as_nanos();
        let key = LocalKey::parse(&format!("{time:x}-{:x}-{serial:x}", std::process::id()))?;
        let path = file_path(root, &key);
        match crate::prefs::new_private_file(&path) {
            Ok(file) => return Ok((key, path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    Err(LocalFailure::Exhausted)
}
fn private_entry(path: &Path, directory: bool) -> Result<(), LocalFailure> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink()
        || (directory && !metadata.is_dir())
        || (!directory && !metadata.is_file())
    {
        return Err(LocalFailure::Boundary);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(LocalFailure::Boundary);
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests;
