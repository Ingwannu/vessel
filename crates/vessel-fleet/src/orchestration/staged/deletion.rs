//! Staged same-filesystem deletion journal; full selection save also needs patch backups.
//! A foreign or future journal is never authority to delete an agent directory.

use super::{
    SkillHomes, SkillRoot, SkillService, filesystem,
    patches::{self, Patch},
};
use crate::dir::{Dir, Entry};
use crate::orchestration::SkillError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use vessel_proto::skills::{MAX_SKILLS, SkillName, SkillOperation, SkillSelection, SkillsSnapshot};

const VERSION: u32 = 1;
const RANDOM_BYTES: usize = 8;
const HEX_DIGITS: usize = RANDOM_BYTES.saturating_mul(2);
const PREFIX: &str = "transaction-";
const DOCUMENT: &str = "transaction.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Home {
    Shared,
    Claude,
    LegacyCodex,
}
const HOMES: [Home; 3] = [Home::Shared, Home::Claude, Home::LegacyCodex];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Owner {
    #[serde(rename = "vessel-skills-transaction")]
    Vessel,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Applying,
    Committed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Removal {
    home: Home,
    canonical_root: PathBuf,
    name: SkillName,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    owner: Owner,
    version: u32,
    id: String,
    phase: Phase,
    previous: Option<SkillSelection>,
    next: SkillSelection,
    #[serde(deserialize_with = "super::limits::decode::<_, _, {super::limits::MAX_PATCHES}>")]
    removals: Vec<Removal>,
    #[serde(deserialize_with = "super::limits::decode::<_, _, {super::limits::MAX_PATCHES}>")]
    patches: Vec<Patch>,
}

#[derive(Debug)]
pub(super) struct DeletionTransaction {
    state: Dir,
    journal: Dir,
    roots: BTreeMap<Home, Dir>,
    document: Document,
}

fn root(homes: &SkillHomes, home: Home) -> &SkillRoot {
    match home {
        Home::Shared => &homes.shared,
        Home::Claude => &homes.claude,
        Home::LegacyCodex => &homes.legacy_codex,
    }
}
fn filename(id: &str) -> String {
    format!("{PREFIX}{id}")
}
fn stage(id: &str, skill: &SkillName) -> String {
    format!(
        ".vessel-delete-{id}-{}",
        filesystem::hash(skill.as_str().as_bytes())
    )
}
fn valid_id(id: &str) -> bool {
    id.len() == HEX_DIGITS && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl DeletionTransaction {
    pub(super) fn begin(
        homes: &SkillHomes,
        state: &SkillRoot,
        previous: Option<SkillSelection>,
        next: SkillSelection,
        plan: &SkillsSnapshot,
    ) -> Result<Self, SkillError> {
        let mut random = [0_u8; RANDOM_BYTES];
        getrandom::fill(&mut random).map_err(std::io::Error::other)?;
        let id = format!(
            "{:0width$x}",
            u64::from_le_bytes(random),
            width = HEX_DIGITS
        );
        let state = state.create()?;
        let (roots, removals) = gather(homes, plan)?;
        for home in roots.values() {
            separate(&state, home)?;
        }
        let journal = state.create_dir(&filename(&id))?;
        let document = Document {
            owner: Owner::Vessel,
            version: VERSION,
            id,
            phase: Phase::Applying,
            previous,
            next,
            removals,
            patches: Vec::new(),
        };
        let transaction = Self {
            state,
            journal,
            roots,
            document,
        };
        transaction.write()?;
        Ok(transaction)
    }

    pub(super) fn capture_patches(
        &mut self,
        service: &SkillService,
        plan: &SkillsSnapshot,
    ) -> Result<(), SkillError> {
        self.document.patches = patches::capture(service, plan, &self.journal)?;
        for home in service.homes.roots() {
            if let Some(dir) = home.read()? {
                separate(&self.state, &dir)?;
                // A patch may create a previously absent home; retain its handle.
                if !self.roots.values().any(|root| root.path() == dir.path()) {
                    let slot = HOMES
                        .into_iter()
                        .find(|slot| root(&service.homes, *slot).0 == home.0)
                        .ok_or(SkillError::Recovery)?;
                    let _old = self.roots.insert(slot, dir);
                }
            }
        }
        self.write()
    }

    fn write(&self) -> Result<(), SkillError> {
        // The file cap is checked before a live directory can be staged.
        let bytes = serde_json::to_vec(&self.document)?;
        filesystem::write(&self.journal, &DOCUMENT.parse()?, &bytes)
    }

    pub(super) fn apply(&self) -> Result<(), SkillError> {
        for removal in &self.document.removals {
            let dir = self.roots.get(&removal.home).ok_or(SkillError::Recovery)?;
            let staged = stage(&self.document.id, &removal.name);
            if dir.entry(&staged)? != Entry::Missing {
                return Err(SkillError::Recovery);
            }
            dir.move_entry(removal.name.as_str(), &staged)?;
        }
        Ok(())
    }

    pub(super) fn rollback(self) -> Result<(), SkillError> {
        for patch in self.document.patches.iter().rev() {
            let dir = self
                .roots
                .values()
                .find(|dir| dir.path() == patch.root)
                .ok_or(SkillError::Recovery)?;
            patch.rollback(dir, &self.journal)?;
        }
        for removal in self.document.removals.iter().rev() {
            let dir = self.roots.get(&removal.home).ok_or(SkillError::Recovery)?;
            let staged = stage(&self.document.id, &removal.name);
            match dir.entry(&staged)? {
                Entry::Missing => continue,
                Entry::Dir => {}
                Entry::File | Entry::Symlink | Entry::Other => return Err(SkillError::Recovery),
            }
            if dir.entry(removal.name.as_str())? != Entry::Missing {
                // Preserve both directories. No cleanup drops the staged original.
                return Err(SkillError::RecoveryConflict);
            }
            dir.move_entry(&staged, removal.name.as_str())?;
        }
        self.discard()
    }

    pub(super) fn commit(mut self) -> Result<(), SkillError> {
        self.document.phase = Phase::Committed;
        self.write()?;
        self.finish()
    }

    fn finish(self) -> Result<(), SkillError> {
        for removal in &self.document.removals {
            let dir = self.roots.get(&removal.home).ok_or(SkillError::Recovery)?;
            filesystem::remove_directory(dir, &stage(&self.document.id, &removal.name))?;
        }
        self.discard()
    }
    fn discard(self) -> Result<(), SkillError> {
        filesystem::remove_directory(&self.journal, "backups")?;
        self.journal.remove_file(DOCUMENT)?;
        self.state.remove_dir(&filename(&self.document.id))?;
        Ok(())
    }
}

/// The caller supplies today's durable choice; mismatched roots or choices refuse.
pub(super) fn recover(
    homes: &SkillHomes,
    state: &SkillRoot,
    durable: Option<&SkillSelection>,
) -> Result<(), SkillError> {
    let Some(state) = state.read()? else {
        return Ok(());
    };
    let mut names = Vec::new();
    state.each_name(|name| {
        if name.strip_prefix(PREFIX).is_some_and(valid_id) {
            if names.len() == MAX_SKILLS {
                return Err(std::io::Error::other("too many skill transaction journals"));
            }
            names.push(name.to_owned());
        }
        Ok(())
    })?;
    if names.len() > MAX_SKILLS {
        return Err(SkillError::Budget);
    }
    for name in names {
        let journal = state.open_dir(&name)?;
        let Some(bytes) = filesystem::read(&journal, &DOCUMENT.parse()?)? else {
            continue;
        };
        let document: Document = serde_json::from_slice(&bytes)?;
        if document.owner != Owner::Vessel
            || document.version != VERSION
            || !valid_id(&document.id)
            || filename(&document.id) != name
            || document.removals.len() > MAX_SKILLS.saturating_mul(HOMES.len())
        {
            return Err(SkillError::Recovery);
        }
        let mut roots = BTreeMap::new();
        for removal in &document.removals {
            let dir = root(homes, removal.home)
                .read()?
                .ok_or(SkillError::Recovery)?;
            if dir.path() != removal.canonical_root {
                return Err(SkillError::Recovery);
            }
            let _old = roots.insert(removal.home, dir);
        }
        for path in document.patches.iter().map(|patch| &patch.root) {
            let mut found = false;
            for home in HOMES {
                if let Some(dir) = root(homes, home).read()?
                    && dir.path() == path
                {
                    let _old = roots.insert(home, dir);
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(SkillError::Recovery);
            }
        }
        for home in roots.values() {
            separate(&state, home)?;
        }
        let transaction = DeletionTransaction {
            state: state.open_dir(".")?,
            journal,
            roots,
            document,
        };
        if transaction.document.phase == Phase::Committed
            || (durable == Some(&transaction.document.next)
                && transaction.document.previous.as_ref() != Some(&transaction.document.next))
        {
            transaction.finish()?;
        } else if durable == transaction.document.previous.as_ref() {
            transaction.rollback()?;
        } else {
            return Err(SkillError::Recovery);
        }
    }
    Ok(())
}

fn gather(
    homes: &SkillHomes,
    plan: &SkillsSnapshot,
) -> Result<(BTreeMap<Home, Dir>, Vec<Removal>), SkillError> {
    let mut roots = BTreeMap::new();
    let mut removals = Vec::new();
    for home in HOMES {
        let Some(dir) = root(homes, home).read()? else {
            continue;
        };
        if roots.values().any(|known: &Dir| known.path() == dir.path()) {
            continue;
        }
        for op in &plan.ops {
            let SkillOperation::Delete { name } = op else {
                continue;
            };
            match dir.entry(name.as_str())? {
                Entry::Missing => continue,
                Entry::Dir => {}
                Entry::File | Entry::Symlink | Entry::Other => return Err(SkillError::Blocked),
            }
            filesystem::check_directory(&dir, name.as_str())?;
            removals.push(Removal {
                home,
                canonical_root: dir.path().to_path_buf(),
                name: name.clone(),
            });
        }
        let _old = roots.insert(home, dir);
    }
    if removals.len() > MAX_SKILLS.saturating_mul(HOMES.len()) {
        return Err(SkillError::Budget);
    }
    Ok((roots, removals))
}

fn separate(state: &Dir, home: &Dir) -> Result<(), SkillError> {
    if state.path().starts_with(home.path()) || home.path().starts_with(state.path()) {
        return Err(SkillError::Root);
    }
    Ok(())
}
