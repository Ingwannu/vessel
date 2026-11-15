//! Immutable, bounded choices using the agent's existing option vocabulary.

use super::{InvalidLaunchValue, codec};
use crate::{ChoiceValue, OptionChoice, OptionId, OptionValueId};
use serde::{Deserialize, Deserializer, Serialize, de};
use std::{collections::HashSet, sync::Arc};

/// An existing choice whose opaque ids have passed launch-payload bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(as = "OptionChoice"))]
#[serde(transparent)]
pub struct LaunchChoice(OptionChoice);

impl LaunchChoice {
    /// Shared byte bound for option ids and select-value ids.
    pub const MAX_ID_BYTES: usize = 256;
    /// Borrow the agent's choice without permitting an invalid mutation.
    #[must_use]
    pub const fn as_choice(&self) -> &OptionChoice {
        &self.0
    }
}

impl TryFrom<OptionChoice> for LaunchChoice {
    type Error = InvalidLaunchValue;
    fn try_from(value: OptionChoice) -> Result<Self, Self::Error> {
        id(value.id.as_str())?;
        match &value.value {
            ChoiceValue::Select { value } => id(value.as_str())?,
            ChoiceValue::Toggle { .. } => {}
        }
        Ok(Self(value))
    }
}

fn id(value: &str) -> Result<(), InvalidLaunchValue> {
    if value.is_empty()
        || value.len() > LaunchChoice::MAX_ID_BYTES
        || value.chars().any(char::is_control)
    {
        Err(InvalidLaunchValue::ChoiceId)
    } else {
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum EncodedChoice {
    Select { id: OptionId, value: OptionValueId },
    Toggle { id: OptionId, on: bool },
}
impl<'de> Deserialize<'de> for LaunchChoice {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let value = match codec::decode(input)? {
            EncodedChoice::Select { id, value } => OptionChoice {
                id,
                value: ChoiceValue::Select { value },
            },
            EncodedChoice::Toggle { id, on } => OptionChoice {
                id,
                value: ChoiceValue::Toggle { on },
            },
        };
        Self::try_from(value).map_err(de::Error::custom)
    }
}

/// Ordered configuration choices with unique option ids; empty means defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(as = "Vec<LaunchChoice>"))]
#[serde(transparent)]
pub struct LaunchChoices(Arc<[LaunchChoice]>);

impl LaunchChoices {
    /// Most choices applied by one preset/probe.
    pub const MAX_CHOICES: usize = 32;
    /// Read in application order, without mutable access.
    #[must_use]
    pub fn as_slice(&self) -> &[LaunchChoice] {
        &self.0
    }
}
impl TryFrom<Vec<LaunchChoice>> for LaunchChoices {
    type Error = InvalidLaunchValue;
    fn try_from(choices: Vec<LaunchChoice>) -> Result<Self, Self::Error> {
        if choices.len() > Self::MAX_CHOICES {
            return Err(InvalidLaunchValue::Choices);
        }
        let mut seen = HashSet::new();
        if choices.iter().any(|choice| !seen.insert(&choice.0.id)) {
            return Err(InvalidLaunchValue::DuplicateChoice);
        }
        Ok(Self(Arc::from(choices)))
    }
}
impl<'de> Deserialize<'de> for LaunchChoices {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let values = codec::bounded::<D, LaunchChoice, { Self::MAX_CHOICES }>(input)?;
        Self::try_from(values).map_err(de::Error::custom)
    }
}
