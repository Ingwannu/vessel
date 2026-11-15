//! Non-destructive maintenance repairs selected files without deleting a skill.

use super::{SkillService, filesystem, manifest, status};
use crate::dir::{Dir, Entry};
use crate::orchestration::{SkillBundle, SkillError};
use vessel_proto::skills::{SkillName, SkillSelection, SkillsSnapshot, SkillsState};

impl SkillService {
    pub(super) fn reconcile(
        &self,
        selection: &SkillSelection,
    ) -> Result<SkillsSnapshot, SkillError> {
        self.validate_selection(selection)?;
        let names = self.bundles.names()?;
        if !names
            .as_slice()
            .iter()
            .any(|name| status::desired(selection, name))
        {
            return self.status(selection);
        }
        let mut roots = Vec::new();
        for home in self.homes.roots().into_iter().take(super::ACTIVE_TARGETS) {
            let root = home.create()?;
            if roots.iter().any(|known: &Dir| known.path() == root.path()) {
                continue;
            }
            for name in names
                .as_slice()
                .iter()
                .filter(|name| status::desired(selection, name))
            {
                let bundle = self.bundles.get(name).ok_or(SkillError::UnknownSkill)?;
                sync(&root, bundle)?;
            }
            roots.push(root);
        }
        self.retire_legacy(selection, &roots)?;
        self.status(selection)
    }

    pub(super) fn maintain(
        &self,
        selection: &SkillSelection,
    ) -> Result<SkillsSnapshot, SkillError> {
        let status = self.status(selection)?;
        match status.state {
            SkillsState::NotInstalled | SkillsState::UpToDate => Ok(status),
            SkillsState::Drift => self.reconcile(selection),
        }
    }

    fn retire_legacy(&self, selection: &SkillSelection, main: &[Dir]) -> Result<(), SkillError> {
        let Some(legacy) = self.homes.legacy_codex.read()? else {
            return Ok(());
        };
        if main.iter().any(|root| root.path() == legacy.path()) {
            return Ok(());
        }
        for name in self
            .bundles
            .names()?
            .as_slice()
            .iter()
            .filter(|name| status::desired(selection, name))
        {
            retire(&legacy, name)?;
        }
        Ok(())
    }
}

fn sync(root: &Dir, bundle: &SkillBundle) -> Result<(), SkillError> {
    let dir = match root.entry(bundle.name().as_str())? {
        Entry::Missing => root.create_dir(bundle.name().as_str())?,
        Entry::Dir => root.open_dir(bundle.name().as_str())?,
        Entry::File | Entry::Symlink | Entry::Other => return Err(SkillError::Blocked),
    };
    let previous = manifest::read(&dir)?.unwrap_or_default();
    // Preflight every managed path before the first write, including removals.
    for path in previous.keys() {
        let _contents = filesystem::read(&dir, path)?;
    }
    for asset in bundle.files() {
        let _contents = filesystem::read(&dir, asset.path())?;
    }
    for asset in bundle.files() {
        filesystem::write(&dir, asset.path(), asset.bytes())?;
    }
    for (path, hash) in previous {
        if !bundle.files().any(|asset| asset.path() == &path) {
            filesystem::remove_unchanged(&dir, &path, hash)?;
        }
    }
    manifest::write(&dir, bundle)
}

fn retire(root: &Dir, name: &SkillName) -> Result<(), SkillError> {
    match root.entry(name.as_str())? {
        Entry::Missing => return Ok(()),
        Entry::Dir => {}
        Entry::File | Entry::Symlink | Entry::Other => return Err(SkillError::Blocked),
    }
    let dir = root.open_dir(name.as_str())?;
    let Some(files) = manifest::read(&dir)? else {
        return Ok(());
    };
    for (path, hash) in files {
        filesystem::remove_unchanged(&dir, &path, hash)?;
    }
    if let Some(bytes) = filesystem::read(&dir, &manifest::path()?)? {
        filesystem::remove_unchanged(&dir, &manifest::path()?, filesystem::hash(&bytes))?;
    }
    // An unmodified empty legacy directory is retired, while extra files stay.
    let mut any = false;
    dir.each_name(|_name| {
        any = true;
        Ok(())
    })?;
    if !any {
        root.remove_dir(name.as_str())?;
    }
    Ok(())
}
