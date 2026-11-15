//! Complete skill state is derived from bundle bytes, never from selection alone.

use super::{SkillService, filesystem, manifest};
use crate::dir::{Dir, Entry};
use crate::orchestration::{SkillBundle, SkillError};
use vessel_proto::skills::{
    SkillName, SkillNames, SkillOperation, SkillSelection, SkillsSnapshot, SkillsState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presence {
    Missing,
    Current,
    Drift,
}

impl SkillService {
    pub(super) fn status(&self, selection: &SkillSelection) -> Result<SkillsSnapshot, SkillError> {
        self.validate_selection(selection)?;
        let available = self.bundles.names()?;
        let mut installed = Vec::new();
        let mut ops = Vec::new();
        let roots = self
            .homes
            .roots()
            .into_iter()
            .map(super::SkillRoot::read)
            .collect::<Result<Vec<_>, _>>()?;
        for name in available.as_slice() {
            let bundle = self.bundles.get(name).ok_or(SkillError::UnknownSkill)?;
            let states = roots
                .iter()
                .map(|root| inspect(root.as_ref(), name, bundle))
                .collect::<Result<Vec<_>, _>>()?;
            if states.iter().any(|state| *state != Presence::Missing) {
                installed.push(name.clone());
            }
            let main: Vec<_> = states.iter().take(super::ACTIVE_TARGETS).copied().collect();
            if desired(selection, name) {
                let operation = if main.contains(&Presence::Missing) {
                    Some(SkillOperation::Add { name: name.clone() })
                } else if main.contains(&Presence::Drift)
                    || legacy_managed(roots.last().and_then(Option::as_ref), name, &roots)?
                {
                    Some(SkillOperation::Update { name: name.clone() })
                } else {
                    None
                };
                if let Some(operation) = operation {
                    ops.push(operation);
                }
            } else if states.iter().any(|state| *state != Presence::Missing) {
                ops.push(SkillOperation::Delete { name: name.clone() });
            }
        }
        let state = if installed.is_empty() {
            SkillsState::NotInstalled
        } else if ops.is_empty() {
            SkillsState::UpToDate
        } else {
            SkillsState::Drift
        };
        Ok(SkillsSnapshot {
            state,
            ops,
            available,
            installed: SkillNames::new(installed)?,
            selection: selection.clone(),
        })
    }

    pub(super) fn validate_selection(&self, selection: &SkillSelection) -> Result<(), SkillError> {
        if let SkillSelection::Custom { skills } = selection
            && skills
                .as_slice()
                .iter()
                .any(|name| self.bundles.get(name).is_none())
        {
            return Err(SkillError::UnknownSkill);
        }
        Ok(())
    }
}

pub(super) fn desired(selection: &SkillSelection, name: &SkillName) -> bool {
    match selection {
        SkillSelection::All => true,
        SkillSelection::Custom { skills } => skills.contains(name),
    }
}

fn inspect(
    root: Option<&Dir>,
    name: &SkillName,
    bundle: &SkillBundle,
) -> Result<Presence, SkillError> {
    let Some(root) = root else {
        return Ok(Presence::Missing);
    };
    match root.entry(name.as_str())? {
        Entry::Missing => return Ok(Presence::Missing),
        Entry::Dir => {}
        Entry::File | Entry::Symlink | Entry::Other => return Err(SkillError::Blocked),
    }
    let dir = root.open_dir(name.as_str())?;
    for asset in bundle.files() {
        if filesystem::read(&dir, asset.path())?.as_deref() != Some(asset.bytes()) {
            return Ok(Presence::Drift);
        }
    }
    Ok(Presence::Current)
}

fn legacy_managed(
    legacy: Option<&Dir>,
    name: &SkillName,
    roots: &[Option<Dir>],
) -> Result<bool, SkillError> {
    let Some(legacy) = legacy else {
        return Ok(false);
    };
    if roots
        .iter()
        .take(super::ACTIVE_TARGETS)
        .flatten()
        .any(|root| root.path() == legacy.path())
    {
        return Ok(false);
    }
    match legacy.entry(name.as_str())? {
        Entry::Missing => Ok(false),
        Entry::Dir => Ok(manifest::read(&legacy.open_dir(name.as_str())?)?.is_some()),
        Entry::File | Entry::Symlink | Entry::Other => Err(SkillError::Blocked),
    }
}
