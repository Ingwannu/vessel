//! Draft identity and validation fail before directory I/O or provider spawning.

use super::*;
use vessel_proto::workspace::WorkspaceRoot;
use vessel_proto::{ChoiceValue, HarnessKind, OptionChoice, OptionId, ProjectId};

fn config() -> DraftCommandConfig {
    DraftCommandConfig {
        harness: HarnessKind::ClaudeCode.into(),
        profile: None,
        root: WorkspaceRoot::Project {
            project: ProjectId::random(),
        },
        directory: None,
        model: None,
        options: Vec::new(),
    }
}

#[test]
fn cache_identity_uses_root_directory_profile_model_and_ordered_choices() {
    let original = config();
    let first = DraftConfigHash::of(&original).unwrap();
    assert_eq!(first, DraftConfigHash::of(&original.clone()).unwrap());
    let mut changed = original.clone();
    changed.root = WorkspaceRoot::Project {
        project: ProjectId::random(),
    };
    assert_ne!(first, DraftConfigHash::of(&changed).unwrap());
    let mut changed = original.clone();
    changed.directory = Some("src".parse().unwrap());
    assert_ne!(first, DraftConfigHash::of(&changed).unwrap());
    let mut changed = original.clone();
    changed.profile = Some("other".parse().unwrap());
    assert_ne!(first, DraftConfigHash::of(&changed).unwrap());
    let mut changed = original.clone();
    changed.model = Some(vessel_proto::ModelId::new("other"));
    assert_ne!(first, DraftConfigHash::of(&changed).unwrap());
    let mut changed = original;
    changed.options = vec![choice("mode"), choice("effort")];
    let before = DraftConfigHash::of(&changed).unwrap();
    assert_ne!(first, before);
    changed.options.reverse();
    assert_ne!(before, DraftConfigHash::of(&changed).unwrap());
}

fn choice(id: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new(id),
        value: ChoiceValue::Toggle { on: true },
    }
}

#[test]
fn duplicate_and_oversized_choice_sets_are_refused_before_work() {
    let mut draft = config();
    assert!(validate(&draft).is_ok());
    draft.options = vec![choice("same"), choice("same")];
    assert_eq!(
        validate(&draft).unwrap_err().code,
        ErrorCode::InvalidRequest
    );
    draft.options = (0..MAX_DRAFT_CHOICES)
        .map(|index| choice(&format!("choice-{index}")))
        .collect();
    assert!(validate(&draft).is_ok());
    draft.options.push(choice("one-more"));
    let error = validate(&draft).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidRequest);
    assert!(error.message.contains(&MAX_DRAFT_CHOICES.to_string()));
}

#[test]
fn routed_catalogs_wait_for_an_independent_immutable_route_proof() {
    use vessel_harness::parse_profiles;
    let profiles = parse_profiles(
        "[profiles.routed]\nharness = \"codex\"\nmodel = \"route-a\"\n",
        None,
    )
    .unwrap();
    let selected = crate::transfers::export::named_in(
        &profiles,
        Some(HarnessKind::Codex),
        Some(&"routed".parse().unwrap()),
    )
    .unwrap();
    assert_eq!(
        cacheable_profile(&selected).unwrap_err().code,
        ErrorCode::Unsupported
    );
    assert!(cacheable_profile(&vessel_harness::Profile::default_for(HarnessKind::Codex)).is_ok());
    assert_eq!(profile_changed().code, ErrorCode::Unavailable);
}

#[test]
fn profile_selection_uses_the_supplied_snapshot_and_keeps_harness_validation() {
    use vessel_harness::parse_profiles;
    let old = parse_profiles(
        "[profiles.work]\nharness = \"codex\"\nargs = [\"old-config\"]\n",
        None,
    )
    .unwrap();
    let new = parse_profiles(
        "[profiles.work]\nharness = \"codex\"\nargs = [\"new-config\"]\n",
        None,
    )
    .unwrap();
    let name = "work".parse().unwrap();
    let before =
        crate::transfers::export::named_in(&old, Some(HarnessKind::Codex), Some(&name)).unwrap();
    let after =
        crate::transfers::export::named_in(&new, Some(HarnessKind::Codex), Some(&name)).unwrap();
    assert_ne!(before, after);
    assert!(
        crate::transfers::export::named_in(&old, Some(HarnessKind::ClaudeCode), Some(&name))
            .is_err()
    );
}

#[test]
fn secret_backed_added_catalogs_wait_for_the_separate_runtime_secret_proof() {
    let agents = vessel_harness::parse_agents(
        "[agents.custom]\nprogram = \"custom-acp\"\nenv = { KEY = { secret = \"fixture-key\" } }\n",
        None,
    )
    .unwrap();
    let agent = agents.get(&"custom".parse().unwrap()).unwrap();
    let profile = vessel_harness::Profile::default_added(Arc::clone(agent));
    assert_eq!(
        cacheable_profile(&profile).unwrap_err().code,
        ErrorCode::Unsupported
    );
}
