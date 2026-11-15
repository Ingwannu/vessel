//! Reference naming/dedup plus stricter actual-option/account bounds.

use super::fixtures::*;
use super::{MigrationCatalogues, build_migration_plan};
use crate::launch_profiles::LaunchProfileClientError;
use vessel_proto::launch_profiles::{
    LaunchAccount, LaunchAccountKind, LaunchChoices, LaunchProfile, LaunchSelection,
};
use vessel_proto::{HarnessKind, LaunchProfileId, NodeId, OptionCategory};

#[test]
fn existing_first_pair_id_and_model_selection_dedup_preserve_manual_records() {
    let selected = profile(
        "My Zero",
        vec![
            choice("reported-model", "m0", OptionCategory::Model),
            choice("reported-mode", "plan", OptionCategory::Mode),
        ],
    );
    let existing = snapshot(3, vec![selected.clone()]);
    let plan = build_migration_plan(
        node(),
        &existing,
        &favorites(&["m0", "m1", "m0"]),
        &catalogues(),
    )
    .unwrap();
    assert_eq!(plan.expected().get(), 3);
    assert_eq!(plan.profiles().as_slice().first(), Some(&selected));
    assert_eq!(plan.profiles().as_slice().len(), 2);
    let imported = plan.profiles().as_slice().last().unwrap();
    assert_eq!(imported.display_name().as_str(), "One");
    assert_eq!(
        imported.id(),
        LaunchProfileId::for_legacy_favorite(&favorite("m1")).unwrap()
    );
    assert_eq!(imported.launch().options().as_slice().len(), 1);
    assert_eq!(plan.added(), &[imported.id()]);
    let manually_changed = LaunchProfile::new(
        LaunchProfileId::for_legacy_favorite(&favorite("m0")).unwrap(),
        "Manual change".parse().unwrap(),
        LaunchSelection::new(HarnessKind::Gemini.into(), None, LaunchChoices::default()),
    );
    let kept = snapshot(4, vec![manually_changed.clone()]);
    let empty: MigrationCatalogues = Vec::new().try_into().unwrap();
    let plan = build_migration_plan(node(), &kept, &favorites(&["m0"]), &empty).unwrap();
    assert!(plan.added().is_empty());
    assert_eq!(plan.profiles().as_slice(), &[manually_changed]);
}

#[test]
fn explicit_default_selector_is_equivalent_only_when_the_node_reports_it() {
    let explicit = LaunchProfile::new(
        LaunchProfileId::random(),
        "Existing".parse().unwrap(),
        LaunchSelection::new(
            HarnessKind::Codex.into(),
            Some("codex".parse().unwrap()),
            vec![choice("reported-model", "m0", OptionCategory::Model)]
                .try_into()
                .unwrap(),
        ),
    );
    let old = snapshot(0, vec![explicit]);
    let plan = build_migration_plan(node(), &old, &favorites(&["m0"]), &catalogues()).unwrap();
    assert!(plan.added().is_empty());
    let named = vec![catalogue(
        node(),
        LaunchAccount::new("other-account".parse().unwrap(), LaunchAccountKind::Named),
    )]
    .try_into()
    .unwrap();
    let plan = build_migration_plan(node(), &old, &favorites(&["m0"]), &named).unwrap();
    assert_eq!(plan.added().len(), 1);
}

#[test]
fn source_name_candidates_and_suffixes_are_exact_and_case_sensitive() {
    let names = [
        "Zero",
        "Provider · Zero",
        "Provider · Zero (m0)",
        "Provider · Zero (m0) 2",
    ];
    let old = snapshot(
        0,
        names
            .into_iter()
            .map(|name| profile(name, Vec::new()))
            .collect(),
    );
    let plan = build_migration_plan(node(), &old, &favorites(&["m0"]), &catalogues()).unwrap();
    assert_eq!(
        plan.profiles()
            .as_slice()
            .last()
            .unwrap()
            .display_name()
            .as_str(),
        "Provider · Zero (m0) 3"
    );
    let old = snapshot(0, vec![profile("zero", Vec::new())]);
    let plan = build_migration_plan(node(), &old, &favorites(&["m0"]), &catalogues()).unwrap();
    assert_eq!(
        plan.profiles()
            .as_slice()
            .last()
            .unwrap()
            .display_name()
            .as_str(),
        "Zero"
    );
}

#[test]
fn unresolved_models_and_cross_machine_replies_never_form_a_partial_plan() {
    let old = snapshot(0, Vec::new());
    assert!(matches!(
        build_migration_plan(
            node(),
            &old,
            &favorites(&["m0", "no-native-alias-is-invented"]),
            &catalogues()
        ),
        Err(LaunchProfileClientError::LegacyModelUnavailable(_))
    ));
    let other: NodeId = "00000000-0000-4000-8000-000000000002".parse().unwrap();
    let stale = vec![catalogue(
        other,
        LaunchAccount::new("codex".parse().unwrap(), LaunchAccountKind::Default),
    )]
    .try_into()
    .unwrap();
    assert!(matches!(
        build_migration_plan(node(), &old, &favorites(&["m0"]), &stale),
        Err(LaunchProfileClientError::LegacyCatalogueMismatch)
    ));
}
