//! A single native option observation becomes one provider catalogue. No modes,
//! models or per-model efforts are fabricated when the producer reports none.

use super::ProviderDefinition;
use vessel_proto::{
    ModelId, OptionCategory, OptionKind, SessionOption,
    providers::{
        ApprovalFlow, Catalogue, CatalogueError, DefaultChoice, Mode, ModeCatalogue, ModeRisk,
        Model, ModelSpec, Models, ThinkingChoice,
    },
};

/// Native observations must be unambiguous and respect the protocol's bounds.
#[derive(Debug, thiserror::Error)]
pub enum CatalogueOptionsError {
    /// More than one selector for one category needs a provider-specific decoder.
    #[error("provider reported multiple selectors for one catalogue category")]
    Ambiguous,
    /// Toggle/unknown controls cannot describe a selectable catalogue category.
    #[error("provider catalogue category is not a select option")]
    NotSelect,
    /// The current thinking/mode choice must actually exist in the observation.
    #[error("provider selected a choice absent from its catalogue")]
    MissingCurrent,
    /// Model/effort definition refused its bounded shape.
    #[error(transparent)]
    Models(#[from] CatalogueError),
    /// Mode IDs/defaults refused their bounded shape.
    #[error(transparent)]
    Modes(#[from] vessel_proto::providers::InvalidProvider),
    /// Native labels are retained only when safe for display.
    #[error(transparent)]
    Label(#[from] vessel_proto::agents::InvalidAgentText),
}

/// Turn one probe into one catalogue without a second model/mode process.
/// Runtime native IDs remain native; this is not a permission-mode translator.
///
/// # Errors
/// Ambiguous controls, inconsistent current selections and invalid values fail
/// the entire observation instead of silently omitting choices.
pub fn catalogue_from_options(
    definition: &ProviderDefinition,
    options: &[SessionOption],
) -> Result<Catalogue, CatalogueOptionsError> {
    let mut model_selector = None;
    let mut permission_selector = None;
    let mut thinking_selector = None;
    for option in options {
        let target = match option.category {
            OptionCategory::Model => Some(&mut model_selector),
            OptionCategory::Mode => Some(&mut permission_selector),
            OptionCategory::ThoughtLevel => Some(&mut thinking_selector),
            OptionCategory::ModelConfig | OptionCategory::Other => None,
        };
        if let Some(target) = target
            && target.replace(option).is_some()
        {
            return Err(CatalogueOptionsError::Ambiguous);
        }
    }
    let permission_modes = mode_catalogue(definition, permission_selector)?;
    let (thinking, default_thinking) = thinking_choices(thinking_selector)?;
    let available_models = if let Some(option) = model_selector {
        let (current, values) = selected(option)?;
        values
            .iter()
            .map(|value| {
                let spec = ModelSpec {
                    id: ModelId::new(value.id.as_str()),
                    label: value.name.parse()?,
                    default: Some(if &value.id == current {
                        DefaultChoice::Yes
                    } else {
                        DefaultChoice::No
                    }),
                    visibility: None,
                    thinking: thinking.clone(),
                    default_thinking: default_thinking.clone(),
                };
                Model::new(spec).map_err(CatalogueOptionsError::from)
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    Ok(Catalogue {
        models: Models::new(available_models)?,
        modes: permission_modes,
    })
}

fn selected(
    option: &SessionOption,
) -> Result<(&vessel_proto::OptionValueId, &[vessel_proto::OptionValue]), CatalogueOptionsError> {
    match &option.kind {
        OptionKind::Select { current, values } => {
            if !values.iter().any(|value| &value.id == current) {
                return Err(CatalogueOptionsError::MissingCurrent);
            }
            Ok((current, values))
        }
        OptionKind::Toggle { .. } | OptionKind::Unknown => Err(CatalogueOptionsError::NotSelect),
    }
}

fn mode_catalogue(
    definition: &ProviderDefinition,
    option: Option<&SessionOption>,
) -> Result<ModeCatalogue, CatalogueOptionsError> {
    let Some(option) = option else {
        return Ok(ModeCatalogue::new(vec![], None)?);
    };
    let (current, values) = selected(option)?;
    let modes = values
        .iter()
        .map(|value| {
            let id = value.id.as_str().parse()?;
            let known = definition.modes.modes().iter().find(|mode| mode.id == id);
            Ok(Mode {
                id,
                label: value.name.parse()?,
                risk: known.map_or(ModeRisk::Moderate, |m| m.risk),
                approval: known.map_or(ApprovalFlow::Interactive, |m| m.approval),
            })
        })
        .collect::<Result<Vec<_>, CatalogueOptionsError>>()?;
    Ok(ModeCatalogue::new(modes, Some(current.as_str().parse()?))?)
}

type Thinking = (
    Option<Vec<ThinkingChoice>>,
    Option<vessel_proto::OptionValueId>,
);

fn thinking_choices(option: Option<&SessionOption>) -> Result<Thinking, CatalogueOptionsError> {
    let Some(option) = option else {
        return Ok((None, None));
    };
    let (current, values) = selected(option)?;
    let choices = values
        .iter()
        .map(|value| {
            Ok(ThinkingChoice {
                id: value.id.clone(),
                label: value.name.parse()?,
                default: Some(if &value.id == current {
                    DefaultChoice::Yes
                } else {
                    DefaultChoice::No
                }),
            })
        })
        .collect::<Result<Vec<_>, CatalogueOptionsError>>()?;
    Ok((Some(choices), Some(current.clone())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::ProviderRegistry;
    use vessel_proto::{OptionId, OptionValue, OptionValueId, providers::ProviderOverrides};

    fn option(category: OptionCategory, id: &str) -> SessionOption {
        SessionOption {
            id: OptionId::new(id),
            name: id.into(),
            description: None,
            category,
            kind: OptionKind::Select {
                current: OptionValueId::new(id),
                values: vec![OptionValue {
                    id: OptionValueId::new(id),
                    name: id.into(),
                    description: None,
                    group: None,
                }],
            },
        }
    }

    #[test]
    fn one_observation_keeps_native_modes_and_model_efforts_without_fallbacks() {
        let registry = ProviderRegistry::build(&ProviderOverrides::default(), &[]).unwrap();
        let definition = registry.get(&"codex".parse().unwrap()).unwrap();
        let empty = catalogue_from_options(definition, &[]).unwrap();
        assert!(empty.models.all().is_empty());
        assert!(empty.modes.modes().is_empty());
        let catalogue = catalogue_from_options(
            definition,
            &[
                option(OptionCategory::Mode, "native-mode"),
                option(OptionCategory::Model, "native-model"),
                option(OptionCategory::ThoughtLevel, "high"),
            ],
        )
        .unwrap();
        assert_eq!(
            catalogue.modes.default_mode().unwrap().as_str(),
            "native-mode"
        );
        let model = catalogue.models.all().first().unwrap().spec();
        assert_eq!(model.id.as_str(), "native-model");
        assert_eq!(model.default_thinking.as_ref().unwrap().as_str(), "high");
        assert_eq!(
            catalogue.modes.modes().first().unwrap().approval,
            ApprovalFlow::Interactive
        );
    }

    #[test]
    fn duplicate_selectors_and_missing_current_choices_fail_the_whole_observation() {
        let registry = ProviderRegistry::build(&ProviderOverrides::default(), &[]).unwrap();
        let definition = registry.get(&"claude".parse().unwrap()).unwrap();
        let first = option(OptionCategory::Mode, "plan");
        assert!(matches!(
            catalogue_from_options(definition, &[first.clone(), first.clone()]),
            Err(CatalogueOptionsError::Ambiguous)
        ));
        let mut missing = first;
        missing.kind = OptionKind::Select {
            current: OptionValueId::new("missing"),
            values: vec![],
        };
        assert!(matches!(
            catalogue_from_options(definition, &[missing]),
            Err(CatalogueOptionsError::MissingCurrent)
        ));
    }
}
