//! Pure draft and live transitions; only known-invalid saved modes are omitted.

use super::LaunchProfileClientError;
use vessel_proto::launch_profiles::{LaunchCatalogueOptions, LaunchChoices, LaunchProfile};
use vessel_proto::{
    ChoiceValue, OptionCategory, OptionKind, SessionMode, SessionOption, SessionSpec,
};

/// A complete live preparation, or a request to wait for authoritative options.
#[derive(Debug)]
#[must_use]
pub enum LivePreparation {
    /// An unknown/unread mode catalogue must not silently clear a permission.
    AwaitingOptions,
    /// Concrete choices and explicit omitted saved modes.
    Ready(PreparedLiveProfile),
}

/// A ready bundle contains no provider/account/process override.
#[derive(Debug)]
#[must_use]
pub struct PreparedLiveProfile {
    options: LaunchChoices,
    omitted_modes: LaunchChoices,
}
impl PreparedLiveProfile {
    /// Model, mode, thinking, then features; stable within each category.
    #[must_use]
    pub const fn options(&self) -> &LaunchChoices {
        &self.options
    }
    /// Only retired saved modes, left unchanged in the running process.
    #[must_use]
    pub const fn omitted_modes(&self) -> &LaunchChoices {
        &self.omitted_modes
    }
}

/// Copy into an ordinary creation draft, preserving its folder/title/place.
/// Omitted account means the normal node-selected default. Creation remains
/// strict: unsupported choices are not removed, and no live id binding is kept.
#[must_use]
pub fn materialize_draft(profile: &LaunchProfile, mut draft: SessionSpec) -> SessionSpec {
    let selected = profile.launch();
    draft.harness = selected.harness().clone();
    draft.profile = selected.account_profile().cloned();
    let mut choices = selected.options().as_slice().to_vec();
    choices.sort_by_key(|choice| rank(choice.category()));
    draft.mode = SessionMode::Structured {
        model: None,
        resume: None,
        options: choices
            .into_iter()
            .map(vessel_proto::launch_profiles::LaunchChoice::into_choice)
            .collect(),
    };
    draft
}

/// Prepare a preset obtained from the node's eligibility read, using an
/// authoritative current catalogue. No provider/default-account table lives
/// here, and this pure transformation grants no authority to apply anything.
///
/// # Errors
/// The bounded output invariant failed; no request has been sent.
pub fn prepare_live(
    profile: &LaunchProfile,
    live: Option<&LaunchCatalogueOptions>,
) -> Result<LivePreparation, LaunchProfileClientError> {
    let Some(live) = live else {
        return Ok(LivePreparation::AwaitingOptions);
    };
    let needs_mode = profile.launch().options().as_slice().iter().any(|choice| {
        choice.category() == OptionCategory::Mode
            || live.as_slice().iter().any(|option| {
                option.id == choice.as_choice().id && option.category == OptionCategory::Mode
            })
    });
    if needs_mode
        && live.as_slice().iter().any(|option| {
            option.category == OptionCategory::Mode && matches!(option.kind, OptionKind::Unknown)
        })
    {
        return Ok(LivePreparation::AwaitingOptions);
    }
    let mut options = Vec::new();
    let mut omitted = Vec::new();
    for choice in profile.launch().options().as_slice() {
        let reported = live
            .as_slice()
            .iter()
            .find(|option| option.id == choice.as_choice().id);
        let category = reported.map_or(choice.category(), |option| option.category);
        if category == OptionCategory::Mode || choice.category() == OptionCategory::Mode {
            if reported.is_some_and(|option| matches!(option.kind, OptionKind::Unknown)) {
                return Ok(LivePreparation::AwaitingOptions);
            }
            if !reported.is_some_and(|option| {
                option.category == OptionCategory::Mode
                    && offered(option, &choice.as_choice().value)
            }) {
                omitted.push(choice.clone());
                continue;
            }
        }
        options.push(choice.clone().with_category(category));
    }
    options.sort_by_key(|choice| rank(choice.category()));
    Ok(LivePreparation::Ready(PreparedLiveProfile {
        options: options.try_into()?,
        omitted_modes: omitted.try_into()?,
    }))
}

const fn rank(category: OptionCategory) -> u8 {
    match category {
        OptionCategory::Model => 0,
        OptionCategory::Mode => 1,
        OptionCategory::ThoughtLevel => 2,
        OptionCategory::ModelConfig | OptionCategory::Other => 3,
    }
}
fn offered(option: &SessionOption, choice: &ChoiceValue) -> bool {
    match (&option.kind, choice) {
        (OptionKind::Select { values, .. }, ChoiceValue::Select { value }) => {
            values.iter().any(|offered| offered.id == *value)
        }
        (OptionKind::Toggle { .. }, ChoiceValue::Toggle { .. }) => true,
        (OptionKind::Select { .. }, ChoiceValue::Toggle { .. })
        | (OptionKind::Toggle { .. }, ChoiceValue::Select { .. })
        | (OptionKind::Unknown, ChoiceValue::Select { .. } | ChoiceValue::Toggle { .. }) => false,
    }
}
