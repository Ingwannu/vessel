//! Capture the exact file write set before applying a frozen skill plan.
//! Backups belong to one private transaction, never the fleet GC namespace.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use vessel_proto::skills::{SkillName, SkillsSnapshot};
use vessel_proto::{BlobHash, BundlePath};

use super::{SkillService, filesystem, manifest, mode::SavedMode, reconcile::update_name};
use crate::dir::{Dir, Entry};
use crate::orchestration::{SkillBundle, SkillError};

/// Includes both old and new catalogue bytes, not arbitrary user-added files.
const MAX_BACKUP_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Backup {
    hash: BlobHash,
    mode: SavedMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    path: BundlePath,
    previous: Option<Backup>,
    applied: Option<BlobHash>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Patch {
    pub(super) root: std::path::PathBuf,
    name: SkillName,
    existed: bool,
    changes: Vec<Change>,
}

impl Patch {
    fn capture(
        root: &Dir,
        bundle: &SkillBundle,
        journal: &Dir,
        budget: &mut usize,
        legacy: bool,
    ) -> Result<Option<Self>, SkillError> {
        let dir = match root.entry(bundle.name().as_str())? {
            Entry::Missing => None,
            Entry::Dir => Some(root.open_dir(bundle.name().as_str())?),
            Entry::File | Entry::Symlink | Entry::Other => return Err(SkillError::Blocked),
        };
        let previous = dir.as_ref().map(manifest::read).transpose()?.flatten();
        if legacy && previous.is_none() {
            return Ok(None);
        }
        let mut targets = BTreeMap::new();
        for (path, owned) in previous.unwrap_or_default() {
            let current = dir
                .as_ref()
                .map(|dir| filesystem::read(dir, &path))
                .transpose()?
                .flatten();
            let hash = current.as_deref().map(filesystem::hash);
            let _old = targets.insert(path, hash.filter(|hash| *hash != owned));
        }
        if !legacy {
            for asset in bundle.files() {
                let _old =
                    targets.insert(asset.path().clone(), Some(filesystem::hash(asset.bytes())));
            }
        }
        let _old = targets.insert(
            manifest::path()?,
            if legacy {
                None
            } else {
                Some(filesystem::hash(&manifest::encoded(bundle)?))
            },
        );
        let changes = targets
            .into_iter()
            .map(|(path, applied)| {
                let original = dir
                    .as_ref()
                    .map(|dir| filesystem::snapshot(dir, &path))
                    .transpose()?
                    .flatten();
                let previous = original
                    .map(|original| {
                        *budget = budget.saturating_add(original.bytes.len());
                        if *budget > MAX_BACKUP_BYTES {
                            return Err(SkillError::Budget);
                        }
                        let hash = filesystem::hash(&original.bytes);
                        filesystem::write(journal, &backup_path(hash)?, &original.bytes)?;
                        Ok(Backup {
                            hash,
                            mode: original.mode,
                        })
                    })
                    .transpose()?;
                Ok(Change {
                    path,
                    previous,
                    applied,
                })
            })
            .collect::<Result<_, SkillError>>()?;
        Ok(Some(Self {
            root: root.path().to_path_buf(),
            name: bundle.name().clone(),
            existed: dir.is_some(),
            changes,
        }))
    }

    /// A mismatched external edit leaves its live content and all backups intact.
    pub(super) fn rollback(&self, root: &Dir, journal: &Dir) -> Result<(), SkillError> {
        if root.path() != self.root {
            return Err(SkillError::Recovery);
        }
        let dir = match root.entry(self.name.as_str())? {
            Entry::Missing if !self.existed => return Ok(()),
            Entry::Missing => root.create_dir(self.name.as_str())?,
            Entry::Dir => root.open_dir(self.name.as_str())?,
            Entry::File | Entry::Symlink | Entry::Other => {
                return Err(SkillError::RecoveryConflict);
            }
        };
        for change in self.changes.iter().rev() {
            change.rollback(&dir, journal)?;
        }
        if !self.existed {
            filesystem::prune_empty(&dir)?;
            let mut any = false;
            dir.each_name(|_name| {
                any = true;
                Ok(())
            })?;
            if !any {
                root.remove_dir(self.name.as_str())?;
            }
        }
        Ok(())
    }
}

impl Change {
    fn rollback(&self, dir: &Dir, journal: &Dir) -> Result<(), SkillError> {
        let current = filesystem::snapshot(dir, &self.path)?;
        let hash = current.as_ref().map(|state| filesystem::hash(&state.bytes));
        let prior = self.previous.as_ref().map(|backup| backup.hash);
        // The original state means the apply did not reach this file, or rollback
        // already restored it. Also compare mode so chmods are not overwritten.
        if hash == prior
            && current.as_ref().map(|state| state.mode)
                == self.previous.as_ref().map(|backup| backup.mode)
        {
            return Ok(());
        }
        if hash != self.applied
            || current
                .as_ref()
                .is_some_and(|state| Some(state.mode) != SavedMode::private().ok())
        {
            return Err(SkillError::RecoveryConflict);
        }
        match &self.previous {
            Some(backup) => {
                let bytes = filesystem::read(journal, &backup_path(backup.hash)?)?
                    .ok_or(SkillError::Recovery)?;
                if filesystem::hash(&bytes) != backup.hash {
                    return Err(SkillError::Recovery);
                }
                filesystem::restore(
                    dir,
                    &self.path,
                    &filesystem::FileState {
                        bytes,
                        mode: backup.mode,
                    },
                )
            }
            None => {
                if let Some(hash) = self.applied {
                    filesystem::remove_unchanged(dir, &self.path, hash)?;
                }
                Ok(())
            }
        }
    }
}

fn backup_path(hash: BlobHash) -> Result<BundlePath, SkillError> {
    Ok(format!("backups/{hash}").parse()?)
}

pub(super) fn capture(
    service: &SkillService,
    plan: &SkillsSnapshot,
    journal: &Dir,
) -> Result<Vec<Patch>, SkillError> {
    let names: Vec<_> = plan.ops.iter().filter_map(update_name).collect();
    let mut patches = Vec::new();
    let mut roots = Vec::new();
    let mut budget = 0;
    for (index, home) in service.homes.roots().into_iter().enumerate() {
        let root = if index < super::ACTIVE_TARGETS && !names.is_empty() {
            Some(home.create()?)
        } else {
            home.read()?
        };
        let Some(root) = root else {
            continue;
        };
        if roots.iter().any(|known: &Dir| known.path() == root.path()) {
            continue;
        }
        for name in &names {
            let bundle = service.bundles.get(name).ok_or(SkillError::UnknownSkill)?;
            if let Some(patch) = Patch::capture(
                &root,
                bundle,
                journal,
                &mut budget,
                index >= super::ACTIVE_TARGETS,
            )? {
                patches.push(patch);
            }
        }
        roots.push(root);
    }
    Ok(patches)
}
