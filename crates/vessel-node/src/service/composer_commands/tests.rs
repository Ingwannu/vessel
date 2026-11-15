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

mod preparation;
