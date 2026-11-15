//! Staged skill service, pending the shared method table and node controller.
//! Tests exercise the actual filesystem machinery against isolated agent homes.

mod filesystem;
mod manifest;
mod mode;
mod reconcile;
mod status;

/// Shared and Claude are active; Codex only retires its dedicated old copy.
const ACTIVE_TARGETS: usize = 2;

use std::path::{Component, PathBuf};

use super::{SkillBundles, SkillError};
use crate::dir::Dir;

#[derive(Debug, Clone)]
struct SkillRoot(PathBuf);
impl SkillRoot {
    fn new(path: PathBuf) -> Result<Self, SkillError> {
        if path.file_name().is_none()
            || !path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
        {
            return Err(SkillError::Root);
        }
        Ok(Self(path))
    }
    fn read(&self) -> Result<Option<Dir>, SkillError> {
        match std::fs::canonicalize(&self.0) {
            Ok(path) => Ok(Some(Dir::open_root(&path)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
    fn create(&self) -> Result<Dir, SkillError> {
        crate::files::private_dir(&self.0)?;
        self.read()?.ok_or(SkillError::Blocked)
    }
}

#[derive(Debug)]
struct SkillHomes {
    shared: SkillRoot,
    claude: SkillRoot,
    legacy_codex: SkillRoot,
}
impl SkillHomes {
    fn roots(&self) -> [&SkillRoot; 3] {
        [&self.shared, &self.claude, &self.legacy_codex]
    }
}

#[derive(Debug)]
struct SkillService {
    bundles: SkillBundles,
    homes: SkillHomes,
}

#[cfg(test)]
mod tests;
