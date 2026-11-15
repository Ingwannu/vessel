//! Pi model/state decoding follows providers/pi/agent.ts:1122-1178 and
//! rpc-types.ts. Native thinking capabilities filter the fixed RPC vocabulary;
//! no UI catalogue is advertised before the native observation exists.

use serde::Deserialize;
use std::collections::BTreeMap;
use vessel_proto::{
    ModelId, OptionValueId,
    providers::{
        Catalogue, DefaultChoice, MAX_CATALOGUE_ENTRIES, MAX_CHOICE_ID, ModeCatalogue, Model,
        ModelSpec, Models, ThinkingChoice, ThinkingChoices,
    },
};

const MAX_CATALOGUE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

impl Level {
    const ALL: [Self; 7] = [
        Self::Off,
        Self::Minimal,
        Self::Low,
        Self::Medium,
        Self::High,
        Self::Xhigh,
        Self::Max,
    ];
    const fn id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
        }
    }
    const fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Minimal => "Minimal",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Xhigh => "XHigh",
            Self::Max => "Max",
        }
    }
}

#[derive(Deserialize)]
struct ModelsResponse {
    models: Vec<NativeModel>,
}

#[derive(Deserialize)]
struct State {
    model: Option<NativeModel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeModel {
    provider: String,
    id: String,
    name: Option<String>,
    #[serde(default)]
    reasoning: bool,
    #[serde(default)]
    thinking_level_map: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum InvalidCatalogue {
    #[error("Pi catalogue exceeds its resource bound")]
    TooLarge,
    #[error("Pi catalogue native observation was invalid")]
    Native,
}

pub(super) fn decode(models: &[u8], state: &[u8]) -> Result<Catalogue, InvalidCatalogue> {
    if models.len() > MAX_CATALOGUE_BYTES || state.len() > MAX_CATALOGUE_BYTES {
        return Err(InvalidCatalogue::TooLarge);
    }
    let ModelsResponse { models } =
        serde_json::from_slice(models).map_err(|_invalid| InvalidCatalogue::Native)?;
    let state: State =
        serde_json::from_slice(state).map_err(|_invalid| InvalidCatalogue::Native)?;
    if models.len() > MAX_CATALOGUE_ENTRIES {
        return Err(InvalidCatalogue::TooLarge);
    }
    let models = models
        .into_iter()
        .map(|model| convert(model, state.model.as_ref()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Catalogue {
        models: Models::new(models).map_err(|_invalid| InvalidCatalogue::Native)?,
        modes: ModeCatalogue::new(vec![], None).map_err(|_invalid| InvalidCatalogue::Native)?,
    })
}

fn convert(
    model: NativeModel,
    configured: Option<&NativeModel>,
) -> Result<Model, InvalidCatalogue> {
    for text in [&model.provider, &model.id] {
        if text.is_empty() || text.len() > MAX_CHOICE_ID || text.chars().any(char::is_control) {
            return Err(InvalidCatalogue::Native);
        }
    }
    if model.provider.contains('/') {
        return Err(InvalidCatalogue::Native);
    }
    let (thinking, default_thinking) = if model.reasoning {
        thinking(&model.thinking_level_map)?
    } else {
        (None, None)
    };
    let is_default =
        configured.is_some_and(|active| active.provider == model.provider && active.id == model.id);
    let label = format!(
        "{}/{}",
        model.provider,
        model
            .name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(&model.id)
    );
    Model::new(ModelSpec {
        id: ModelId::new(format!("{}/{}", model.provider, model.id)),
        label: label.parse().map_err(|_invalid| InvalidCatalogue::Native)?,
        default: Some(if is_default {
            DefaultChoice::Yes
        } else {
            DefaultChoice::No
        }),
        visibility: None,
        thinking,
        default_thinking,
    })
    .map_err(|_invalid| InvalidCatalogue::Native)
}

fn thinking(
    map: &BTreeMap<String, Option<String>>,
) -> Result<(Option<ThinkingChoices>, Option<OptionValueId>), InvalidCatalogue> {
    let supported = Level::ALL
        .into_iter()
        .filter(|level| match map.get(level.id()) {
            Some(None) => false,
            Some(Some(_native)) => true,
            None => !matches!(level, Level::Xhigh | Level::Max),
        })
        .collect::<Vec<_>>();
    // Pi clamps upward from medium first, then to the highest lower level.
    let default = supported
        .iter()
        .find(|level| **level >= Level::Medium)
        .or_else(|| supported.last())
        .copied()
        .ok_or(InvalidCatalogue::Native)?;
    let choices = supported
        .into_iter()
        .map(|level| {
            Ok(ThinkingChoice {
                id: OptionValueId::new(level.id()),
                label: level
                    .label()
                    .parse()
                    .map_err(|_invalid| InvalidCatalogue::Native)?,
                default: Some(if level == default {
                    DefaultChoice::Yes
                } else {
                    DefaultChoice::No
                }),
            })
        })
        .collect::<Result<Vec<_>, InvalidCatalogue>>()?;
    Ok((
        Some(ThinkingChoices::new(choices).map_err(|_invalid| InvalidCatalogue::Native)?),
        Some(OptionValueId::new(default.id())),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_map_filters_extended_efforts_and_clamps_upward_then_downward() {
        let model = br#"{"models":[{"provider":"vendor","id":"native/model","reasoning":true,"thinkingLevelMap":{"medium":null,"xhigh":"extra-high","max":null}},{"provider":"vendor","id":"plain"}]}"#;
        let catalogue = decode(
            model,
            br#"{"model":{"provider":"vendor","id":"native/model"}}"#,
        )
        .unwrap();
        assert_eq!(
            catalogue.models.all()[0].spec().id.as_str(),
            "vendor/native/model"
        );
        assert_eq!(
            catalogue.models.all()[0]
                .spec()
                .default_thinking
                .as_ref()
                .unwrap()
                .as_str(),
            "high"
        );
        let choices = catalogue.models.all()[0].spec().thinking.as_ref().unwrap();
        assert!(choices.contains(&OptionValueId::new("xhigh")));
        assert!(!choices.contains(&OptionValueId::new("max")));
        assert!(!choices.contains(&OptionValueId::new("medium")));
        assert_eq!(
            catalogue.models.all()[0].spec().default,
            Some(DefaultChoice::Yes)
        );
        assert!(catalogue.models.all()[1].spec().thinking.is_none());
        assert!(catalogue.modes.modes().is_empty());
        let lower = br#"{"models":[{"provider":"vendor","id":"small","reasoning":true,"thinkingLevelMap":{"medium":null,"high":null}}]}"#;
        assert_eq!(
            decode(lower, b"{}").unwrap().models.all()[0]
                .spec()
                .default_thinking
                .as_ref()
                .unwrap()
                .as_str(),
            "low"
        );
    }
    #[test]
    fn malformed_or_inconsistent_native_data_never_becomes_an_empty_success() {
        assert_eq!(
            decode(&vec![0; MAX_CATALOGUE_BYTES + 1], b"{}"),
            Err(InvalidCatalogue::TooLarge)
        );
        assert!(decode(b"{}", b"{}").is_err());
        assert!(decode(br#"{"models":[{"provider":"","id":"x"}]}"#, b"{}").is_err());
        assert!(
            decode(
                br#"{"models":[{"provider":"bad/provider","id":"x"}]}"#,
                b"{}"
            )
            .is_err()
        );
        assert!(decode(br#"{"models":[{"provider":"vendor","id":"x","reasoning":true,"thinkingLevelMap":{"off":null,"minimal":null,"low":null,"medium":null,"high":null}}]}"#, b"{}").is_err());
        assert!(decode(br#"{"models":[]}"#, b"{}").is_ok());
    }
}
