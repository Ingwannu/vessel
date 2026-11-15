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

/// Charge decoded owned metadata across every page, not only model count.
/// Otherwise individually bounded frames could accumulate gigabytes before the
/// final checked model conversion. Unknown native fields are not retained.
const MAX_NATIVE_METADATA_BYTES: usize = 4 * 1024 * 1024;

#[derive(Default)]
pub(super) struct MetadataBudget(usize);

impl MetadataBudget {
    pub(super) fn admit(
        &mut self,
        page: &ModelPage,
    ) -> Result<(), super::catalogue::CatalogueProbeError> {
        let owned = page.data.iter().fold(
            page.next_cursor.as_ref().map_or(0, String::capacity),
            |bytes, model| bytes.saturating_add(model.owned_bytes()),
        );
        self.0 = self
            .0
            .checked_add(owned)
            .filter(|bytes| *bytes <= MAX_NATIVE_METADATA_BYTES)
            .ok_or(super::catalogue::CatalogueProbeError::Invalid)?;
        Ok(())
    }
}

impl NativeModel {
    fn owned_bytes(&self) -> usize {
        let efforts = self
            .supported_reasoning_efforts
            .as_ref()
            .map_or(0, |efforts| {
                efforts.iter().fold(
                    efforts.capacity().saturating_mul(size_of::<Effort>()),
                    |bytes, effort| bytes.saturating_add(effort.reasoning_effort.capacity()),
                )
            });
        size_of::<Self>()
            .saturating_add(self.id.capacity())
            .saturating_add(self.display_name.as_ref().map_or(0, String::capacity))
            .saturating_add(
                self.default_reasoning_effort
                    .as_ref()
                    .map_or(0, String::capacity),
            )
            .saturating_add(efforts)
    }
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
    let has_known_default = defaults
        .model
        .as_ref()
        .is_some_and(|configured| native.iter().any(|model| &model.id == configured));
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
            let is_default = if has_known_default {
                Some(defaults.model.as_ref() == Some(&native.id))
            } else {
                native.is_default
            };
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
    #[test]
    fn a_retired_configured_model_does_not_erase_the_native_default() {
        let page: ModelPage = serde_json::from_str(
            r#"{"data":[{"id":"a","isDefault":false},{"id":"b","isDefault":true}]}"#,
        )
        .unwrap();
        let config: ConfigResponse =
            serde_json::from_str(r#"{"config":{"model":"retired"}}"#).unwrap();
        let observed = models(page.data, &config.config).unwrap();
        assert_eq!(observed.all()[0].spec().default, Some(DefaultChoice::No));
        assert_eq!(observed.all()[1].spec().default, Some(DefaultChoice::Yes));
    }

    #[test]
    fn individually_small_valid_native_fields_cannot_accumulate_unbounded_pages() {
        let mut budget = MetadataBudget::default();
        let mut refused = false;
        for number in 0..64 {
            let data = (0..64)
                .map(|model| {
                    let efforts: Vec<_> = (0..32)
                        .map(|effort| Effort {
                            reasoning_effort: format!("effort-{effort:02}-{}", "x".repeat(48)),
                        })
                        .collect();
                    NativeModel {
                        id: format!("model-{number}-{model}"),
                        display_name: Some("Native model".into()),
                        is_default: Some(false),
                        default_reasoning_effort: Some(efforts[0].reasoning_effort.clone()),
                        supported_reasoning_efforts: Some(efforts),
                    }
                })
                .collect();
            let page = ModelPage {
                data,
                next_cursor: None,
            };
            if budget.admit(&page).is_err() {
                refused = true;
                break;
            }
        }
        assert!(
            refused,
            "model-count and per-frame limits alone do not bound retained metadata"
        );
    }
}
