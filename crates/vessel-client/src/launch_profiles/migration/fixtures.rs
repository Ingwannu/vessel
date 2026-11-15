//! Explicit fictional node catalogues for pure planner and state-machine tests.

use super::{MigrationCatalogue, MigrationCatalogues};
use vessel_proto::launch_profiles::{
    LaunchAccount, LaunchAccountKind, LaunchCatalogue, LaunchChoice, LaunchChoices, LaunchProfile,
    LaunchProfileRevision, LaunchProfilesSnapshot, LaunchSelection, LegacyFavorite,
    LegacyFavorites,
};
use vessel_proto::{
    ChoiceValue, HarnessKind, LaunchProfileId, NodeId, OptionCategory, OptionChoice, OptionId,
    OptionKind, OptionValue, OptionValueId, SessionOption,
};

pub(super) fn node() -> NodeId {
    "00000000-0000-4000-8000-000000000001".parse().unwrap()
}
pub(super) fn favorite(model: &str) -> LegacyFavorite {
    LegacyFavorite::new("provider".parse().unwrap(), model.parse().unwrap())
}
pub(super) fn favorites(models: &[&str]) -> LegacyFavorites {
    models
        .iter()
        .map(|model| favorite(model))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}
pub(super) fn choice(id: &str, value: &str, category: OptionCategory) -> LaunchChoice {
    LaunchChoice::try_from(OptionChoice {
        id: OptionId::new(id),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    })
    .unwrap()
    .with_category(category)
}
pub(super) fn profile(name: &str, choices: Vec<LaunchChoice>) -> LaunchProfile {
    LaunchProfile::new(
        LaunchProfileId::random(),
        name.parse().unwrap(),
        LaunchSelection::new(HarnessKind::Codex.into(), None, choices.try_into().unwrap()),
    )
}
pub(super) fn snapshot(revision: u32, profiles: Vec<LaunchProfile>) -> LaunchProfilesSnapshot {
    LaunchProfilesSnapshot::new(
        LaunchProfileRevision::new(revision),
        profiles.try_into().unwrap(),
    )
}
pub(super) fn catalogue(for_node: NodeId, account: LaunchAccount) -> MigrationCatalogue {
    let model = SessionOption {
        id: OptionId::new("reported-model"),
        name: "Models".to_owned(),
        description: None,
        category: OptionCategory::Model,
        kind: OptionKind::Select {
            current: OptionValueId::new("m0"),
            values: [("m0", "Zero"), ("m1", "One")]
                .into_iter()
                .map(|(id, label)| OptionValue {
                    id: OptionValueId::new(id),
                    name: label.to_owned(),
                    description: None,
                    group: None,
                })
                .collect(),
        },
    };
    let selector = match account.kind() {
        LaunchAccountKind::Default | LaunchAccountKind::Unknown => None,
        LaunchAccountKind::Named => Some(account.account_name().clone()),
    };
    let catalogue = LaunchCatalogue::new(
        LaunchSelection::new(
            HarnessKind::Codex.into(),
            selector,
            LaunchChoices::default(),
        ),
        account,
        vec![model].try_into().unwrap(),
        chrono::DateTime::UNIX_EPOCH,
    )
    .unwrap();
    MigrationCatalogue::new(
        for_node,
        "provider".parse().unwrap(),
        "Provider".parse().unwrap(),
        catalogue,
    )
    .unwrap()
}
pub(super) fn ready() -> MigrationCatalogue {
    catalogue(
        node(),
        LaunchAccount::new("codex".parse().unwrap(), LaunchAccountKind::Default),
    )
}
pub(super) fn catalogues() -> MigrationCatalogues {
    vec![ready()].try_into().unwrap()
}
