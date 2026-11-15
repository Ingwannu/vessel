//! Codex model/config response boundary. These shapes follow Paseo's
//! codex-app-server-agent.ts fetchNativeModels and buildCodexModelDefinition.
//! Unlike its permissive empty-list fallback, malformed required data refuses
//! the observation. Unknown additive fields are ignored.

use serde::Deserialize;
use vessel_proto::{
    ModelId, OptionValueId,
    providers::{
        CatalogueError, DefaultChoice, Model, ModelSpec, Models, ThinkingChoice, ThinkingChoices,
    },
};

#[derive(Deserialize)]
pub(super) struct ModelPage {
    pub(super) data: Vec<NativeModel>,
    #[serde(default, rename = "nextCursor")]
    pub(super) next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NativeModel {
    id: String,
    display_name: Option<String>,
    is_default: Option<bool>,
    default_reasoning_effort: Option<String>,
    supported_reasoning_efforts: Option<Vec<Effort>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Effort {
    reasoning_effort: String,
}

#[derive(Default, Deserialize)]
pub(super) struct ConfigResponse {
    pub(super) config: Defaults,
}

#[derive(Default, Deserialize)]
pub(super) struct Defaults {
    model: Option<String>,
    #[serde(alias = "modelReasoningEffort")]
    model_reasoning_effort: Option<String>,
}

impl Defaults {
    pub(super) fn fallback(self, fallback: Self) -> Self {
        Self {
            model: self.model.or(fallback.model),
            model_reasoning_effort: self
                .model_reasoning_effort
                .or(fallback.model_reasoning_effort),
        }
    }
}

pub(super) fn models(
    native: Vec<NativeModel>,
    defaults: &Defaults,
) -> Result<Models, super::catalogue::CatalogueProbeError> {
    let models = native
        .into_iter()
        .map(|native| {
            let default_effort = defaults
                .model_reasoning_effort
                .as_ref()
                .or(native.default_reasoning_effort.as_ref());
            let mut choices = Vec::<ThinkingChoice>::new();
            for effort in native.supported_reasoning_efforts.into_iter().flatten() {
                // Duplicate native IDs use the reference's first-order/last-value
                // map semantics; their label is the ID, so retaining first is equal.
                if choices
                    .iter()
                    .any(|choice| choice.id.as_str() == effort.reasoning_effort)
                {
                    continue;
                }
                choices.push(choice(&effort.reasoning_effort, default_effort)?);
            }
            if let Some(default) = default_effort
                && !choices.iter().any(|choice| choice.id.as_str() == default)
            {
                choices.push(choice(default, default_effort)?);
            }
            let default_thinking = default_effort
                .map(|effort| OptionValueId::new(effort.as_str()))
                .or_else(|| choices.first().map(|choice| choice.id.clone()));
            let label = native
                .display_name
                .filter(|label| !label.trim().is_empty())
                .unwrap_or_else(|| native.id.clone());
            let is_default = defaults
                .model
                .as_ref()
                .map_or(native.is_default, |model| Some(model == &native.id));
            Ok(Model::new(ModelSpec {
                id: ModelId::new(native.id),
                label: label
                    .parse()
                    .map_err(|_invalid| super::catalogue::CatalogueProbeError::Invalid)?,
                default: is_default.map(|yes| {
                    if yes {
                        DefaultChoice::Yes
                    } else {
                        DefaultChoice::No
                    }
                }),
                visibility: None,
                thinking: if choices.is_empty() {
                    None
                } else {
                    Some(ThinkingChoices::new(choices)?)
                },
                default_thinking,
            })?)
        })
        .collect::<Result<Vec<_>, super::catalogue::CatalogueProbeError>>()?;
    Ok(Models::new(models)?)
}

fn choice(
    id: &str,
    default: Option<&String>,
) -> Result<ThinkingChoice, super::catalogue::CatalogueProbeError> {
    Ok(ThinkingChoice {
        id: OptionValueId::new(id),
        label: id
            .parse()
            .map_err(|_invalid| super::catalogue::CatalogueProbeError::Invalid)?,
        default: Some(if default.is_some_and(|default| default == id) {
            DefaultChoice::Yes
        } else {
            DefaultChoice::No
        }),
    })
}

impl From<CatalogueError> for super::catalogue::CatalogueProbeError {
    fn from(_invalid: CatalogueError) -> Self {
        Self::Invalid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configured_defaults_preserve_model_specific_efforts() {
        let page: ModelPage = serde_json::from_str(r#"{"data":[{"id":"a","displayName":"A","isDefault":true,"defaultReasoningEffort":"high","supportedReasoningEfforts":[{"reasoningEffort":"high"},{"reasoningEffort":"high"}]},{"id":"b","defaultReasoningEffort":"low","supportedReasoningEfforts":[{"reasoningEffort":"low"}]}]}"#).unwrap();
        let defaults: ConfigResponse = serde_json::from_str(r#"{"config":{"model":"b"}}"#).unwrap();
        let observed = models(page.data, &defaults.config).unwrap();
        assert_eq!(observed.all()[0].spec().default, Some(DefaultChoice::No));
        assert_eq!(observed.all()[1].spec().default, Some(DefaultChoice::Yes));
        assert_eq!(
            observed.all()[0]
                .spec()
                .thinking
                .as_ref()
                .unwrap()
                .all()
                .len(),
            1
        );
        assert_eq!(
            observed.all()[1]
                .spec()
                .default_thinking
                .as_ref()
                .unwrap()
                .as_str(),
            "low"
        );
    }
    #[test]
    fn malformed_catalogue_never_becomes_successful_empty_discovery() {
        assert!(serde_json::from_str::<ModelPage>("{}").is_err());
        assert!(
            serde_json::from_str::<ModelPage>(r#"{"data":[{"displayName":"missing id"}]}"#)
                .is_err()
        );
        let page: ModelPage = serde_json::from_str(r#"{"data":[{"id":"bad\n"}]}"#).unwrap();
        assert!(models(page.data, &Defaults::default()).is_err());
    }
}
