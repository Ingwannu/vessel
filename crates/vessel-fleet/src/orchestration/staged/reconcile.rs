//! Non-destructive maintenance applies only a frozen plan's selected additions/updates.

use super::{SkillService, filesystem, manifest};
use crate::dir::{Dir, Entry};
use crate::orchestration::{SkillBundle, SkillError};
use vessel_proto::skills::{SkillName, SkillOperation, SkillsSnapshot};

#[cfg(test)]
use vessel_proto::skills::{SkillSelection, SkillsState};

impl SkillService {
    #[cfg(test)]
    pub(super) fn reconcile(
        &mut self,
        selection: &SkillSelection,
    ) -> Result<SkillsSnapshot, SkillError> {
        let plan = self.status(selection)?;
        self.apply_updates(&plan)?;
        self.status(selection)
    }

    /// A save transaction captures this exact plan, so no rescan may widen writes.
    pub(super) fn apply_updates(&mut self, plan: &SkillsSnapshot) -> Result<(), SkillError> {
        let names = plan.ops.iter().filter_map(update_name).collect::<Vec<_>>();
        if names.is_empty() {
            return Ok(());
        }
        for name in &names {
            let _bundle = self.bundles.get(name).ok_or(SkillError::UnknownSkill)?;
        }
        let mut roots = Vec::new();
        for home in self.homes.roots().into_iter().take(super::ACTIVE_TARGETS) {
            let root = home.create()?;
            if roots.iter().any(|known: &Dir| known.path() == root.path()) {
                continue;
            }
            for name in &names {
                let bundle = self.bundles.get(name).ok_or(SkillError::UnknownSkill)?;
                sync(&root, bundle)?;
            }
            roots.push(root);
        }
        self.retire_legacy(&names, &roots)
    }

    #[cfg(test)]
    pub(super) fn maintain(
        &mut self,
        selection: &SkillSelection,
    ) -> Result<SkillsSnapshot, SkillError> {
        let plan = self.status(selection)?;
        match plan.state {
            SkillsState::NotInstalled | SkillsState::UpToDate => Ok(plan),
            SkillsState::Drift => {
                self.apply_updates(&plan)?;
                self.status(selection)
            }
        }
    }

    fn retire_legacy(&self, names: &[&SkillName], main: &[Dir]) -> Result<(), SkillError> {
        let Some(legacy) = self.homes.legacy_codex.read()? else {
            return Ok(());
        };
        if main.iter().any(|root| root.path() == legacy.path()) {
            return Ok(());
        }
        for name in names {
            retire(&legacy, name)?;
        }
        Ok(())
    }
}

pub(super) fn update_name(operation: &SkillOperation) -> Option<&SkillName> {
    match operation {
        SkillOperation::Add { name } | SkillOperation::Update { name } => Some(name),
        SkillOperation::Delete { .. } => None,
    }
}

fn sync(root: &Dir, bundle: &SkillBundle) -> Result<(), SkillError> {
    let dir = match root.entry(bundle.name().as_str())? {
        Entry::Missing => root.create_dir(bundle.name().as_str())?,
        Entry::Dir => root.open_dir(bundle.name().as_str())?,
        Entry::File | Entry::Symlink | Entry::Other => return Err(SkillError::Blocked),
    };
    let previous = manifest::read(&dir)?.unwrap_or_default();
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
