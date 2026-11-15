//! Source-profile transitions with authoritative controls, without provider tables.

use super::materialize::*;
use vessel_proto::launch_profiles::{LaunchChoice, LaunchSelection};
use vessel_proto::{
    ChoiceValue, OptionCategory, OptionChoice, OptionKind, SessionMode, SessionOption, SessionSpec,
};
use vessel_proto::{
    HarnessKind, LaunchProfileId, OptionId, OptionValue, OptionValueId, ProfileName,
};

fn choice(id: &str, value: &str, category: OptionCategory) -> LaunchChoice {
    LaunchChoice::try_from(OptionChoice {
        id: OptionId::new(id),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    })
    .unwrap()
    .with_category(category)
}
fn profile(choices: Vec<LaunchChoice>) -> vessel_proto::launch_profiles::LaunchProfile {
    vessel_proto::launch_profiles::LaunchProfile::new(
        LaunchProfileId::random(),
        "preset".parse().unwrap(),
        LaunchSelection::new(HarnessKind::Codex.into(), None, choices.try_into().unwrap()),
    )
}
fn reported(id: &str, category: OptionCategory, value: &str) -> SessionOption {
    SessionOption {
        id: OptionId::new(id),
        name: id.to_owned(),
        description: None,
        category,
        kind: OptionKind::Select {
            current: OptionValueId::new(value),
            values: vec![OptionValue {
                id: OptionValueId::new(value),
                name: value.to_owned(),
                description: None,
                group: None,
            }],
        },
    }
}

#[test]
fn draft_copies_selection_without_live_binding_and_keeps_creation_strict() {
    let preset = profile(vec![choice(
        "retired-mode",
        "retired",
        OptionCategory::Mode,
    )]);
    let draft = SessionSpec::new(
        HarnessKind::Gemini,
        SessionMode::Structured {
            model: Some(vessel_proto::ModelId::new("previous")),
            resume: None,
            options: Vec::new(),
        },
    )
    .with_profile("old-account".parse::<ProfileName>().unwrap())
    .with_cwd("/fixture");
    let prepared = materialize_draft(&preset, draft);
    assert_eq!(prepared.harness, HarnessKind::Codex);
    assert_eq!(prepared.profile, None);
    assert_eq!(
        prepared.cwd.as_deref(),
        Some(std::path::Path::new("/fixture"))
    );
    let SessionMode::Structured {
        model,
        resume,
        options,
    } = &prepared.mode
    else {
        panic!("not structured")
    };
    assert_eq!((model, resume), (&None, &None));
    assert_eq!(
        options,
        &[preset.launch().options().as_slice()[0].as_choice().clone()]
    );
    // Replacing stored metadata later cannot mutate the ordinary draft.
    let _replacement = preset.with_notes(Some("changed guidance".parse().unwrap()));
    assert_eq!(options[0].id.as_str(), "retired-mode");
}

#[test]
fn unread_and_unknown_mode_catalogues_defer_the_whole_application() {
    let preset = profile(vec![choice("mode", "plan", OptionCategory::Mode)]);
    assert!(matches!(
        prepare_live(&preset, None).unwrap(),
        LivePreparation::AwaitingOptions
    ));
    let mut mode = reported("mode", OptionCategory::Mode, "plan");
    mode.kind = OptionKind::Unknown;
    let catalogue = vec![mode].try_into().unwrap();
    assert!(matches!(
        prepare_live(&preset, Some(&catalogue)).unwrap(),
        LivePreparation::AwaitingOptions
    ));
}

#[test]
fn only_known_invalid_saved_modes_are_omitted_and_reported() {
    let preset = profile(vec![
        choice("mode", "retired", OptionCategory::Mode),
        choice("model", "unsupported-model", OptionCategory::Model),
        choice("feature", "unsupported-feature", OptionCategory::Other),
    ]);
    let catalogue = vec![
        reported("mode", OptionCategory::Mode, "current"),
        reported("model", OptionCategory::Model, "available"),
    ]
    .try_into()
    .unwrap();
    let LivePreparation::Ready(prepared) = prepare_live(&preset, Some(&catalogue)).unwrap() else {
        panic!("not ready")
    };
    assert_eq!(
        prepared
            .omitted_modes()
            .as_slice()
            .iter()
            .map(|v| v.as_choice().id.as_str())
            .collect::<Vec<_>>(),
        ["mode"]
    );
    assert_eq!(
        prepared
            .options()
            .as_slice()
            .iter()
            .map(|v| v.as_choice().id.as_str())
            .collect::<Vec<_>>(),
        ["model", "feature"]
    );
    // A complete empty catalogue is known, unlike an unread catalogue.
    let empty = Vec::new().try_into().unwrap();
    let LivePreparation::Ready(prepared) = prepare_live(&preset, Some(&empty)).unwrap() else {
        panic!("not ready")
    };
    assert_eq!(prepared.omitted_modes().as_slice().len(), 1);
}

#[test]
fn reported_categories_order_steps_and_feature_order_is_stable() {
    let preset = profile(vec![
        choice("f1", "on", OptionCategory::Other),
        choice("thinking", "deep", OptionCategory::Other),
        choice("mode", "plan", OptionCategory::Other),
        choice("f2", "on", OptionCategory::ModelConfig),
        choice("model", "m", OptionCategory::Other),
    ]);
    let catalogue = vec![
        reported("mode", OptionCategory::Mode, "plan"),
        reported("model", OptionCategory::Model, "m"),
        reported("thinking", OptionCategory::ThoughtLevel, "deep"),
    ]
    .try_into()
    .unwrap();
    let LivePreparation::Ready(prepared) = prepare_live(&preset, Some(&catalogue)).unwrap() else {
        panic!("not ready")
    };
    assert!(prepared.omitted_modes().as_slice().is_empty());
    assert_eq!(
        prepared
            .options()
            .as_slice()
            .iter()
            .map(|v| v.as_choice().id.as_str())
            .collect::<Vec<_>>(),
        ["model", "mode", "thinking", "f1", "f2"]
    );
}

#[test]
fn retired_mode_ids_reused_by_other_controls_are_not_applied_as_features() {
    let preset = profile(vec![choice("old-mode", "plan", OptionCategory::Mode)]);
    let other = vec![reported("old-mode", OptionCategory::Other, "plan")]
        .try_into()
        .unwrap();
    let LivePreparation::Ready(ready) = prepare_live(&preset, Some(&other)).unwrap() else {
        panic!("not ready")
    };
    assert!(ready.options().as_slice().is_empty());
    assert_eq!(ready.omitted_modes().as_slice().len(), 1);
    let mut unknown = reported("new-mode", OptionCategory::Mode, "plan");
    unknown.kind = OptionKind::Unknown;
    let unknown = vec![unknown].try_into().unwrap();
    assert!(matches!(
        prepare_live(&preset, Some(&unknown)).unwrap(),
        LivePreparation::AwaitingOptions
    ));
}
