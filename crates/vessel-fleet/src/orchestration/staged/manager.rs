//! Public synchronous mechanism; the node supplies a serialized blocking worker.

use std::path::PathBuf;

use vessel_proto::skills::{SkillNames, SkillSelection, SkillsSaved, SkillsSnapshot, SkillsState};

use super::{SkillHomes, SkillRoot, SkillService, deletion, selection::SelectionError};
use crate::orchestration::{SkillBundles, SkillError};

/// Explicit target roots plus private transaction state; never process environment.
#[derive(Debug)]
pub struct SkillLocations {
    homes: SkillHomes,
    state: SkillRoot,
}
impl SkillLocations {
    /// Validate absolute roots without touching any filesystem.
    ///
    /// # Errors
    /// A root is relative, contains traversal, or names a filesystem root.
    pub fn new(
        shared: PathBuf,
        claude: PathBuf,
        legacy_codex: PathBuf,
        state: PathBuf,
    ) -> Result<Self, SkillError> {
        for home in [&shared, &claude, &legacy_codex] {
            if state.starts_with(home) || home.starts_with(&state) {
                return Err(SkillError::Root);
            }
        }
        Ok(Self {
            homes: SkillHomes {
                shared: SkillRoot::new(shared)?,
                claude: SkillRoot::new(claude)?,
                legacy_codex: SkillRoot::new(legacy_codex)?,
            },
            state: SkillRoot::new(state)?,
        })
    }
}

/// Single-owner bundled-skill controller, with no implicit installation at creation.
#[derive(Debug)]
pub struct SkillManager {
    service: SkillService,
    state: SkillRoot,
}
impl SkillManager {
    /// Construct from validated embedded assets and explicitly captured roots.
    #[must_use]
    pub fn new(bundles: SkillBundles, locations: SkillLocations) -> Self {
        Self {
            service: SkillService {
                bundles,
                homes: locations.homes,
            },
            state: locations.state,
        }
    }
    /// Check that a preference names only this release's bundled skills.
    ///
    /// # Errors
    /// Unknown skill names are refused before persistence.
    pub fn validate_selection(&self, selection: &SkillSelection) -> Result<(), SkillError> {
        self.service.validate_selection(selection)
    }
    /// Recover owned journals, then read a complete current snapshot.
    ///
    /// # Errors
    /// Recovery conflicts, unsafe filesystem entries or invalid selection refuse.
    pub fn status(
        &mut self,
        durable: Option<&SkillSelection>,
    ) -> Result<SkillsSnapshot, SkillError> {
        self.service.recover(&self.state, durable)?;
        self.service.status(&durable.cloned().unwrap_or_default())
    }
    /// Install/update selected assets without removing any unselected directory.
    ///
    /// # Errors
    /// Capture, safe filesystem access, convergence or recovery can fail.
    pub fn reconcile(
        &mut self,
        durable: Option<&SkillSelection>,
    ) -> Result<SkillsSnapshot, SkillError> {
        let plan = self.status(durable)?;
        self.update(durable, &plan)
    }
    /// Startup repairs an existing install but does not install into fresh homes.
    ///
    /// # Errors
    /// The same recovery and filesystem errors as [`Self::reconcile`].
    pub fn maintain(
        &mut self,
        durable: Option<&SkillSelection>,
    ) -> Result<SkillsSnapshot, SkillError> {
        let plan = self.status(durable)?;
        match plan.state {
            SkillsState::NotInstalled | SkillsState::UpToDate => Ok(plan),
            SkillsState::Drift => self.update(durable, &plan),
        }
    }
    fn update(
        &mut self,
        durable: Option<&SkillSelection>,
        plan: &SkillsSnapshot,
    ) -> Result<SkillsSnapshot, SkillError> {
        if plan
            .ops
            .iter()
            .all(|op| super::reconcile::update_name(op).is_none())
        {
            return Ok(plan.clone());
        }
        let selection = durable.cloned().unwrap_or_default();
        let mut transaction = deletion::DeletionTransaction::begin(
            &self.service.homes,
            &self.state,
            durable.cloned(),
            selection.clone(),
            &SkillsSnapshot {
                ops: Vec::new(),
                ..plan.clone()
            },
        )?;
        transaction.capture_patches(&self.service, plan)?;
        let result = self
            .service
            .apply_updates(plan)
            .and_then(|()| self.service.status(&selection));
        match result {
            Ok(snapshot) => {
                transaction.commit()?;
                Ok(snapshot)
            }
            Err(error) => {
                transaction.rollback()?;
                Err(error)
            }
        }
    }
    /// Explicitly uninstall all shipped directories while preserving preference.
    ///
    /// # Errors
    /// A blocked directory or recovery conflict is retained for repair.
    pub fn uninstall(
        &mut self,
        durable: Option<&SkillSelection>,
    ) -> Result<SkillsSnapshot, SkillError> {
        let _current = self.status(durable)?;
        let empty = SkillSelection::Custom {
            skills: SkillNames::default(),
        };
        let plan = self.service.status(&empty)?;
        let transaction = deletion::DeletionTransaction::begin(
            &self.service.homes,
            &self.state,
            durable.cloned(),
            durable.cloned().unwrap_or_default(),
            &plan,
        )?;
        if let Err(error) = transaction.apply() {
            transaction.rollback()?;
            return Err(error);
        }
        transaction.commit()?;
        self.service.status(&durable.cloned().unwrap_or_default())
    }
    /// Save after confirming the fresh deletion plan, persisting the choice last.
    ///
    /// # Errors
    /// Typed persistence failure retains a separate rollback result and backups.
    pub fn save<E: std::error::Error + 'static>(
        &mut self,
        durable: Option<SkillSelection>,
        next: &SkillSelection,
        confirmed: &SkillNames,
        persist: impl FnOnce(&SkillSelection) -> Result<(), E>,
    ) -> Result<SkillsSaved, SelectionError<E>> {
        self.service
            .save(&self.state, durable, next, confirmed, persist)
    }
}
