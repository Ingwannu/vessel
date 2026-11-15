//! Model/effort preflight consumes one checked catalogue. A choice offered by a
//! different model is not silently accepted or substituted with a default.

use vessel_proto::{
    ModelId, OptionValueId,
    providers::{Catalogue, DefaultChoice, ModelVisibility},
};

/// The effective model/effort selection, without launch or execution authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSelection {
    /// None leaves the provider's native model default untouched.
    pub model: Option<ModelId>,
    /// None leaves the provider's native thinking default untouched.
    pub thinking: Option<OptionValueId>,
}

/// No raw requested choice appears in an error message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SelectionError {
    /// Explicit unknown models must not silently choose the first entry.
    #[error("requested model is absent from this provider catalogue")]
    ModelUnavailable,
    /// Effort belongs to the selected model, not another model's global list.
    #[error("requested thinking choice is absent from the selected model")]
    ThinkingUnavailable,
}

/// Resolve defaults and check explicit model/effort choices before any start.
/// Compatibility rows may be selected explicitly but never become an implicit
/// first-model fallback. Missing model discovery does not invent a model.
///
/// # Errors
/// Unknown explicit models or thinking choices fail the whole preflight.
pub fn select_model(
    catalogue: &Catalogue,
    requested: Option<&ModelId>,
    thinking: Option<&OptionValueId>,
) -> Result<ModelSelection, SelectionError> {
    let selected = if let Some(requested) = requested {
        Some(
            catalogue
                .models
                .all()
                .iter()
                .find(|model| &model.spec().id == requested)
                .ok_or(SelectionError::ModelUnavailable)?,
        )
    } else {
        let candidates = || {
            catalogue
                .models
                .all()
                .iter()
                .filter(|model| model.spec().visibility != Some(ModelVisibility::Compatibility))
        };
        candidates()
            .find(|model| model.spec().default == Some(DefaultChoice::Yes))
            .or_else(|| candidates().next())
    };
    let Some(model) = selected else {
        if thinking.is_some() {
            return Err(SelectionError::ThinkingUnavailable);
        }
        return Ok(ModelSelection {
            model: None,
            thinking: None,
        });
    };
    let choices = model.spec().thinking.as_ref();
    let selected_thinking = if let Some(requested) = thinking {
        if !choices.is_some_and(|choices| choices.contains(requested)) {
            return Err(SelectionError::ThinkingUnavailable);
        }
        Some(requested.clone())
    } else {
        model.spec().default_thinking.clone().or_else(|| {
            choices.and_then(|choices| {
                choices
                    .all()
                    .iter()
                    .find(|choice| choice.default == Some(DefaultChoice::Yes))
                    .map(|choice| choice.id.clone())
            })
        })
    };
    Ok(ModelSelection {
        model: Some(model.spec().id.clone()),
        thinking: selected_thinking,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::providers::{
        ModeCatalogue, Model, ModelSpec, Models, ThinkingChoice, ThinkingChoices,
    };

    fn model(id: &str, effort: &str) -> Model {
        Model::new(ModelSpec {
            id: ModelId::new(id),
            label: id.parse().unwrap(),
            default: None,
            visibility: None,
            thinking: Some(
                ThinkingChoices::new(vec![ThinkingChoice {
                    id: OptionValueId::new(effort),
                    label: effort.parse().unwrap(),
                    default: Some(DefaultChoice::Yes),
                }])
                .unwrap(),
            ),
            default_thinking: Some(OptionValueId::new(effort)),
        })
        .unwrap()
    }

    #[test]
    fn thinking_validation_uses_the_selected_model_and_unknown_models_do_not_fallback() {
        let catalogue = Catalogue {
            models: Models::new(vec![model("first", "high"), model("second", "low")]).unwrap(),
            modes: ModeCatalogue::new(vec![], None).unwrap(),
        };
        assert_eq!(
            select_model(&catalogue, None, None).unwrap().thinking,
            Some(OptionValueId::new("high"))
        );
        assert_eq!(
            select_model(
                &catalogue,
                Some(&ModelId::new("second")),
                Some(&OptionValueId::new("high"))
            ),
            Err(SelectionError::ThinkingUnavailable)
        );
        assert_eq!(
            select_model(&catalogue, Some(&ModelId::new("missing")), None),
            Err(SelectionError::ModelUnavailable)
        );
        assert_eq!(
            select_model(&catalogue, Some(&ModelId::new("second")), None)
                .unwrap()
                .thinking,
            Some(OptionValueId::new("low"))
        );
    }

    #[test]
    fn absent_discovery_and_hidden_compatibility_are_not_fabricated_choices() {
        let mut hidden = model("compatibility", "high").spec().clone();
        hidden.visibility = Some(ModelVisibility::Compatibility);
        let catalogue = Catalogue {
            models: Models::new(vec![Model::new(hidden).unwrap()]).unwrap(),
            modes: ModeCatalogue::new(vec![], None).unwrap(),
        };
        assert_eq!(select_model(&catalogue, None, None).unwrap().model, None);
        assert_eq!(
            select_model(&catalogue, Some(&ModelId::new("compatibility")), None)
                .unwrap()
                .model,
            Some(ModelId::new("compatibility"))
        );
        assert_eq!(
            select_model(&catalogue, None, Some(&OptionValueId::new("high"))),
            Err(SelectionError::ThinkingUnavailable)
        );
    }
}
