//! Pure append-only migration; existing entries win by pair, id or selection.

use super::super::LaunchProfileClientError;
use super::{MigrationCatalogue, MigrationCatalogues};
use std::{collections::HashSet, sync::Arc};
use vessel_proto::launch_profiles::{
    LaunchAccountKind, LaunchChoice, LaunchChoices, LaunchProfile, LaunchProfileName,
    LaunchProfileRevision, LaunchProfiles, LaunchProfilesSnapshot, LaunchSelection, LegacyFavorite,
    LegacyFavorites,
};
use vessel_proto::{
    ChoiceValue, LaunchProfileId, OptionCategory, OptionChoice, OptionKind, OptionValue,
    SessionOption,
};

/// One whole-list CAS candidate; no mutation or marker write has occurred.
#[derive(Debug)]
#[must_use]
pub struct LegacyMigrationPlan {
    expected: LaunchProfileRevision,
    profiles: LaunchProfiles,
    added: Arc<[LaunchProfileId]>,
}
impl LegacyMigrationPlan {
    /// Revision that must match before this exact whole-list replacement.
    #[must_use]
    pub const fn expected(&self) -> LaunchProfileRevision {
        self.expected
    }
    /// Existing-first list; every existing record remains byte-semantically intact.
    #[must_use]
    pub const fn profiles(&self) -> &LaunchProfiles {
        &self.profiles
    }
    /// Newly appended identities, at most the host list's bound.
    #[must_use]
    pub fn added(&self) -> &[LaunchProfileId] {
        &self.added
    }
}

/// Plan against one actual host read and complete explicit catalogue mappings.
/// Duplicate source pairs, existing ids and existing model selections are
/// skipped. Any unresolved favorite/cap failure refuses the whole plan.
///
/// # Errors
/// Missing/ambiguous model controls, unresolved ids, duplicate/oversized
/// catalogues or profile/name/body bounds. No raw id fallback is fabricated.
pub fn build_migration_plan(
    node: vessel_proto::NodeId,
    existing: &LaunchProfilesSnapshot,
    favorites: &LegacyFavorites,
    catalogues: &MigrationCatalogues,
) -> Result<LegacyMigrationPlan, LaunchProfileClientError> {
    if catalogues
        .as_slice()
        .iter()
        .any(|entry| entry.node() != node)
    {
        return Err(LaunchProfileClientError::LegacyCatalogueMismatch);
    }
    let mut profiles = existing.profiles().as_slice().to_vec();
    let mut ids = profiles
        .iter()
        .map(LaunchProfile::id)
        .collect::<HashSet<_>>();
    let mut names = profiles
        .iter()
        .map(|profile| profile.display_name().clone())
        .collect::<HashSet<_>>();
    let mut pairs = HashSet::new();
    let mut added = Vec::new();
    for favorite in favorites.as_slice() {
        if !pairs.insert(favorite) {
            continue;
        }
        let id = LaunchProfileId::for_legacy_favorite(favorite)?;
        if ids.contains(&id) {
            continue;
        }
        let catalogue = catalogues
            .as_slice()
            .iter()
            .find(|entry| entry.provider() == favorite.provider())
            .ok_or_else(|| {
                LaunchProfileClientError::LegacyCatalogueMissing(favorite.provider().clone())
            })?;
        let (option, model) = model(catalogue, favorite)?;
        if profiles
            .iter()
            .any(|profile| selected(profile, catalogue, option, model))
        {
            continue;
        }
        if profiles.len() == LaunchProfiles::MAX_PROFILES {
            return Err(vessel_proto::launch_profiles::InvalidLaunchValue::Profiles.into());
        }
        let name = unique_name(catalogue, favorite, model, &mut names)?;
        let choice = LaunchChoice::try_from(OptionChoice {
            id: option.id.clone(),
            value: ChoiceValue::Select {
                value: model.id.clone(),
            },
        })?
        .with_category(OptionCategory::Model);
        let choices = LaunchChoices::try_from(vec![choice])?;
        let target = catalogue.catalogue().selection();
        let selection = LaunchSelection::new(
            target.harness().clone(),
            target.account_profile().cloned(),
            choices,
        );
        profiles.push(LaunchProfile::new(id, name, selection));
        let _new = ids.insert(id);
        added.push(id);
    }
    Ok(LegacyMigrationPlan {
        expected: existing.revision(),
        profiles: profiles.try_into()?,
        added: Arc::from(added),
    })
}

fn model<'a>(
    catalogue: &'a MigrationCatalogue,
    favorite: &LegacyFavorite,
) -> Result<(&'a SessionOption, &'a OptionValue), LaunchProfileClientError> {
    let mut controls = catalogue
        .catalogue()
        .options()
        .as_slice()
        .iter()
        .filter(|option| option.category == OptionCategory::Model);
    let Some(control) = controls.next() else {
        return Err(LaunchProfileClientError::LegacyModelUnavailable(
            favorite.clone(),
        ));
    };
    if controls.next().is_some() {
        return Err(LaunchProfileClientError::LegacyModelAmbiguous(
            catalogue.provider().clone(),
        ));
    }
    let OptionKind::Select { values, .. } = &control.kind else {
        return Err(LaunchProfileClientError::LegacyModelUnavailable(
            favorite.clone(),
        ));
    };
    let model = values
        .iter()
        .find(|value| value.id.as_str() == favorite.model().as_str())
        .ok_or_else(|| LaunchProfileClientError::LegacyModelUnavailable(favorite.clone()))?;
    Ok((control, model))
}

fn selected(
    profile: &LaunchProfile,
    catalogue: &MigrationCatalogue,
    option: &SessionOption,
    model: &OptionValue,
) -> bool {
    let selected = profile.launch();
    let actual = catalogue.catalogue();
    if selected.harness() != actual.selection().harness() {
        return false;
    }
    let account_matches = match selected.account_profile() {
        Some(name) => name == actual.account().account_name(),
        None => actual.account().kind() == LaunchAccountKind::Default,
    };
    account_matches
        && selected.options().as_slice().iter().any(|choice| {
            let is_model =
                choice.category() == OptionCategory::Model || choice.as_choice().id == option.id;
            is_model
                && match &choice.as_choice().value {
                    ChoiceValue::Select { value } => *value == model.id,
                    ChoiceValue::Toggle { .. } => false,
                }
        })
}

fn unique_name(
    catalogue: &MigrationCatalogue,
    favorite: &LegacyFavorite,
    model: &OptionValue,
    used: &mut HashSet<LaunchProfileName>,
) -> Result<LaunchProfileName, LaunchProfileClientError> {
    let model = if model.name.trim().is_empty() {
        favorite.model().as_str()
    } else {
        model.name.trim()
    };
    let provider = catalogue.label().as_str();
    let candidates = [
        model.to_owned(),
        format!("{provider} · {model}"),
        format!("{provider} · {model} ({})", favorite.model().as_str()),
    ];
    for candidate in &candidates {
        let name: LaunchProfileName = candidate.parse()?;
        if used.insert(name.clone()) {
            return Ok(name);
        }
    }
    let Some(base) = candidates.last() else {
        return Err(LaunchProfileClientError::LegacyNameCollision);
    };
    // At most MAX_PROFILES names can already be used. This finite range
    // therefore covers the reference suffix rule without an unbounded loop.
    for suffix in 2..=LaunchProfiles::MAX_PROFILES.saturating_add(1) {
        let name: LaunchProfileName = format!("{base} {suffix}").parse()?;
        if used.insert(name.clone()) {
            return Ok(name);
        }
    }
    Err(LaunchProfileClientError::LegacyNameCollision)
}
