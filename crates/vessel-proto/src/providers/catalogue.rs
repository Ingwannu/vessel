//! Validated provider model observations. These describe offered choices only;
//! they contain no executable, environment, secret reference or endpoint.

use super::ModeCatalogue;
use crate::{ModelId, OptionValueId, agents::AgentLabel};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Maximum models or thinking choices retained from one provider result.
pub const MAX_CATALOGUE_ENTRIES: usize = 4096;
/// Maximum bytes in a native model or thinking identifier.
pub const MAX_CHOICE_ID: usize = 1024;

/// Provider model data cannot be represented safely or unambiguously.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CatalogueError {
    /// Native IDs cannot be empty, contain controls or exceed the local bound.
    #[error(
        "catalogue choice id must be nonempty, contain no controls and fit {MAX_CHOICE_ID} bytes"
    )]
    Identifier,
    /// Resource bound applies before retaining an oversized sequence.
    #[error("catalogue list exceeds {MAX_CATALOGUE_ENTRIES} entries")]
    TooMany,
    /// Default effort must be an actual choice.
    #[error("default thinking choice is absent from the model")]
    MissingThinkingDefault,
    /// Duplicate thinking IDs are ambiguous.
    #[error("model repeats a thinking choice id")]
    DuplicateThinking,
}

/// An optional marker is needed because additions may leave a flag unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultChoice {
    /// Provider explicitly marks this as a default.
    Yes,
    /// Provider explicitly removes default status.
    No,
}

/// Compatibility aliases may be recognized but hidden from selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelVisibility {
    /// Available to a person choosing a model.
    Selectable,
    /// Accepted for compatibility without appearing as a choice.
    Compatibility,
}

/// Thinking choice identifiers remain the provider's, without a fixed effort enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingChoice {
    /// Native choice ID.
    pub id: OptionValueId,
    /// Human label from discovery/configuration.
    pub label: AgentLabel,
    /// Missing preserves a base choice's metadata when merging.
    pub default: Option<DefaultChoice>,
}

/// Untrusted fields are validated together by Model::new or deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSpec {
    /// Native identifier; never interpreted as a provider id.
    pub id: ModelId,
    /// Configured models, like Paseo's profile models, require a label.
    pub label: AgentLabel,
    /// Missing is distinct from explicitly clearing the default flag.
    pub default: Option<DefaultChoice>,
    /// Missing allows an addition to enable a hidden compatibility row.
    pub visibility: Option<ModelVisibility>,
    /// Missing preserves the base model's thinking options during merge.
    #[serde(default, deserialize_with = "read_optional_choices")]
    pub thinking: Option<Vec<ThinkingChoice>>,
    /// Selected native effort, when given.
    pub default_thinking: Option<OptionValueId>,
}

/// A model whose native identifiers and thinking choices were checked once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ModelSpec", into = "ModelSpec")]
pub struct Model(ModelSpec);

impl Model {
    /// Validate a discovered/configured model before it enters a catalogue.
    ///
    /// # Errors
    /// `CatalogueError` retains the violated invariant without echoing raw data.
    pub fn new(spec: ModelSpec) -> Result<Self, CatalogueError> {
        identifier(spec.id.as_str())?;
        let choices = spec.thinking.as_deref().unwrap_or_default();
        if choices.len() > MAX_CATALOGUE_ENTRIES {
            return Err(CatalogueError::TooMany);
        }
        let mut seen = BTreeSet::new();
        for choice in choices {
            identifier(choice.id.as_str())?;
            if !seen.insert(&choice.id) {
                return Err(CatalogueError::DuplicateThinking);
            }
        }
        if let Some(default) = &spec.default_thinking {
            identifier(default.as_str())?;
            if spec.thinking.is_some() && !seen.contains(default) {
                return Err(CatalogueError::MissingThinkingDefault);
            }
        }
        Ok(Self(spec))
    }

    /// Validated metadata; consumers should borrow the shared catalogue.
    #[must_use]
    pub const fn spec(&self) -> &ModelSpec {
        &self.0
    }
}

impl TryFrom<ModelSpec> for Model {
    type Error = CatalogueError;
    fn try_from(spec: ModelSpec) -> Result<Self, Self::Error> {
        Self::new(spec)
    }
}

impl From<Model> for ModelSpec {
    fn from(model: Model) -> Self {
        model.0
    }
}

/// A bounded model list retains first-definition ordering for deterministic merges.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "RawModels", into = "Vec<Model>")]
pub struct Models(Vec<Model>);

impl Models {
    /// Bound discovery/configuration before publication; duplicate model IDs are
    /// retained here so normalization can report and retain the first definition.
    ///
    /// # Errors
    /// `CatalogueError`::`TooMany` refuses an oversized list as a whole.
    pub fn new(models: Vec<Model>) -> Result<Self, CatalogueError> {
        if models.len() > MAX_CATALOGUE_ENTRIES {
            return Err(CatalogueError::TooMany);
        }
        Ok(Self(models))
    }

    /// Provider ordering before normalization.
    #[must_use]
    pub fn all(&self) -> &[Model] {
        &self.0
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct RawModels(#[serde(deserialize_with = "read_sequence")] Vec<Model>);

impl TryFrom<RawModels> for Models {
    type Error = CatalogueError;
    fn try_from(raw: RawModels) -> Result<Self, Self::Error> {
        Self::new(raw.0)
    }
}

impl From<Models> for Vec<Model> {
    fn from(models: Models) -> Self {
        models.0
    }
}

/// Models and modes are observed in one transport call, never separate probes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalogue {
    /// Bounded native model/effort definitions.
    pub models: Models,
    /// An empty mode list is a successful observation, not a probe error.
    pub modes: ModeCatalogue,
}

fn identifier(text: &str) -> Result<(), CatalogueError> {
    if text.is_empty() || text.len() > MAX_CHOICE_ID || text.chars().any(char::is_control) {
        return Err(CatalogueError::Identifier);
    }
    Ok(())
}

fn read_optional_choices<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> Result<Option<Vec<ThinkingChoice>>, D::Error> {
    #[derive(Deserialize)]
    struct Choices(#[serde(deserialize_with = "read_sequence")] Vec<ThinkingChoice>);
    Option::<Choices>::deserialize(de).map(|choices| choices.map(|choices| choices.0))
}

fn read_sequence<'de, D, T>(de: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Sequence<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Sequence<T> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "at most {MAX_CATALOGUE_ENTRIES} catalogue entries")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            let mut entries = Vec::new();
            while let Some(entry) = seq.next_element()? {
                if entries.len() == MAX_CATALOGUE_ENTRIES {
                    return Err(serde::de::Error::custom(CatalogueError::TooMany));
                }
                entries.push(entry);
            }
            Ok(entries)
        }
    }
    de.deserialize_seq(Sequence(std::marker::PhantomData))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> ModelSpec {
        ModelSpec {
            id: ModelId::new("native/model"),
            label: "Model".parse().unwrap(),
            default: None,
            visibility: None,
            thinking: None,
            default_thinking: None,
        }
    }

    #[test]
    fn models_validate_native_ids_efforts_and_default_references_on_decode() {
        let valid = Model::new(spec()).unwrap();
        assert_eq!(
            serde_json::from_str::<Model>(&serde_json::to_string(&valid).unwrap()).unwrap(),
            valid
        );
        let mut input = spec();
        input.id = ModelId::new("bad\n");
        assert_eq!(Model::new(input), Err(CatalogueError::Identifier));
        let mut input = spec();
        input.thinking = Some(vec![]);
        input.default_thinking = Some(OptionValueId::new("high"));
        assert_eq!(
            Model::new(input),
            Err(CatalogueError::MissingThinkingDefault)
        );
        let choice = ThinkingChoice {
            id: OptionValueId::new("high"),
            label: "High".parse().unwrap(),
            default: Some(DefaultChoice::Yes),
        };
        let mut input = spec();
        input.thinking = Some(vec![choice.clone(), choice]);
        assert_eq!(Model::new(input), Err(CatalogueError::DuplicateThinking));
    }

    #[test]
    fn an_oversized_list_fails_as_a_whole() {
        let model = Model::new(spec()).unwrap();
        assert!(Models::new(vec![model.clone(); MAX_CATALOGUE_ENTRIES]).is_ok());
        assert_eq!(
            Models::new(vec![model.clone(); MAX_CATALOGUE_ENTRIES + 1]),
            Err(CatalogueError::TooMany)
        );
        let json = serde_json::to_string(&vec![model; MAX_CATALOGUE_ENTRIES + 1]).unwrap();
        assert!(serde_json::from_str::<Models>(&json).is_err());
    }
}
