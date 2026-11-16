//! A bounded validated manifest stores keys and metadata, never arbitrary paths.
use super::{EntryKind, LocalFailure, Store, create, private_entry};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    io::{Read as _, Write as _},
};
use vessel_app_view::composer::{drafts::DraftKey, local::LocalRecord};

const VERSION: u32 = 1;
const FILE: &str = "manifest.json";
const MAX_BYTES: u64 = 1024 * 1024;
/// One byte snapshot retained by an unsent draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeptFile {
    /// Existing canonical draft identity, not a new draft registry.
    pub draft: DraftKey,
    /// Validated key, name, MIME and immutable length.
    pub record: LocalRecord,
    /// A previous process may have sent this input; never automatically resend it.
    pub unconfirmed: bool,
}
/// Validation runs for deserialization too, before any path can be opened.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<KeptFile>", into = "Vec<KeptFile>")]
pub struct Manifest(Vec<KeptFile>);
impl Manifest {
    /// Every retained draft is bounded by the existing tray bound.
    pub const MAX: usize = vessel_app_view::composer::trays::Trays::MAX * 16;
    /// Validate the complete snapshot before committing or collecting bytes.
    ///
    /// # Errors
    /// Refuses oversized or duplicate per-draft entries.
    pub fn new(files: Vec<KeptFile>) -> Result<Self, LocalFailure> {
        if files.len() > Self::MAX {
            return Err(LocalFailure::Manifest);
        }
        let drafts: HashSet<_> = files.iter().map(|file| file.draft).collect();
        if drafts.len() > vessel_app_view::composer::trays::Trays::MAX {
            return Err(LocalFailure::Manifest);
        }
        let mut distinct = HashSet::new();
        for file in &files {
            if !distinct.insert((file.draft, file.record.key.clone())) {
                return Err(LocalFailure::Manifest);
            }
            if files
                .iter()
                .filter(|other| other.draft == file.draft)
                .count()
                > usize::try_from(vessel_proto::AttachmentLimits::DEFAULT.per_prompt)
                    .unwrap_or(usize::MAX)
            {
                return Err(LocalFailure::Manifest);
            }
        }
        Ok(Self(files))
    }
    /// Stable insertion order retained by the composer.
    #[must_use]
    pub fn files(&self) -> &[KeptFile] {
        &self.0
    }
}
impl TryFrom<Vec<KeptFile>> for Manifest {
    type Error = LocalFailure;
    fn try_from(files: Vec<KeptFile>) -> Result<Self, Self::Error> {
        Self::new(files)
    }
}
impl From<Manifest> for Vec<KeptFile> {
    fn from(manifest: Manifest) -> Self {
        manifest.0
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    version: u32,
    manifest: Manifest,
}
impl Store {
    /// Missing state is empty; invalid/unknown state is retained and refused.
    ///
    /// # Errors
    /// Preserves malformed state on disk and returns a typed manifest/I/O error.
    pub fn load_manifest(&self) -> Result<Manifest, LocalFailure> {
        let inner = self.0.lock().map_err(|_poisoned| LocalFailure::Barrier)?;
        private_entry(&inner.root, EntryKind::Directory)?;
        read(&inner.root)
    }

    /// Write and sync before atomic replacement. Callers keep the old manifest
    /// on refusal and must not run collection against a guessed empty snapshot.
    ///
    /// # Errors
    /// A serialization or disk failure retains the previous committed manifest.
    pub fn save_manifest(&self, manifest: &Manifest) -> Result<(), LocalFailure> {
        let inner = self.0.lock().map_err(|_poisoned| LocalFailure::Barrier)?;
        private_entry(&inner.root, EntryKind::Directory)?;
        let _previous = read(&inner.root)?;
        let bytes = serde_json::to_vec(&Stored {
            version: VERSION,
            manifest: manifest.clone(),
        })
        .map_err(|_invalid| LocalFailure::Manifest)?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_BYTES {
            return Err(LocalFailure::Manifest);
        }
        let (_key, temporary, mut output) = create(&inner.root)?;
        let result = output.write_all(&bytes).and_then(|()| output.sync_all());
        drop(output);
        if let Err(error) = result {
            std::fs::remove_file(temporary)?;
            return Err(error.into());
        }
        std::fs::rename(temporary, inner.root.join(FILE))?;
        std::fs::File::open(&inner.root)?.sync_all()?;
        Ok(())
    }
}

/// Called under the same storage barrier by loads, commits and collection.
pub(super) fn read(root: &std::path::Path) -> Result<Manifest, LocalFailure> {
    let path = root.join(FILE);
    if !path.try_exists()? {
        return Ok(Manifest::default());
    }
    private_entry(&path, EntryKind::File)?;
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > MAX_BYTES {
        return Err(LocalFailure::Manifest);
    }
    let mut bytes = Vec::new();
    let _read = file
        .take(MAX_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)?;
    let stored: Stored =
        serde_json::from_slice(&bytes).map_err(|_invalid| LocalFailure::Manifest)?;
    if stored.version != VERSION {
        return Err(LocalFailure::Manifest);
    }
    Ok(stored.manifest)
}
