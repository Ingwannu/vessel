//! What new sessions start with, by machine and agent (the bridge's
//! `defaults` section). Split from `prefs/mod.rs` to keep it within its
//! size budget; the types are re-exported there unchanged.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use vessel_proto::{ChoiceValue, HarnessKind, OptionId, ProfileName};

use super::{MAX_CHOICE_TEXT, MAX_CHOICES, TooMany};

/// What a new session on one machine starts with.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MachineDefaults {
    /// The agent picked first.
    pub agent: Option<HarnessKind>,
    /// What each agent starts with.
    pub agents: BTreeMap<AgentKind, AgentDefaults>,
}

/// One of the four agents as a key (ordered by its wire name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentKind(pub HarnessKind);

impl Ord for AgentKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.name().cmp(other.0.name())
    }
}

impl PartialOrd for AgentKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// What one agent starts with on one machine.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentDefaults {
    /// The profile; `None` is the agent's default.
    pub profile: Option<ProfileName>,
    /// The agent's own options and their values (at most
    /// [`MAX_CHOICES`], each id and value at most [`MAX_CHOICE_TEXT`]).
    pub choices: Choices,
}

/// An agent's kept choices, bounded.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "BTreeMap<OptionId, ChoiceValue>",
    into = "BTreeMap<OptionId, ChoiceValue>"
)]
pub struct Choices(BTreeMap<OptionId, ChoiceValue>);

impl Choices {
    /// The choices.
    #[must_use]
    pub const fn get(&self) -> &BTreeMap<OptionId, ChoiceValue> {
        &self.0
    }
}

impl TryFrom<BTreeMap<OptionId, ChoiceValue>> for Choices {
    type Error = TooMany;

    fn try_from(choices: BTreeMap<OptionId, ChoiceValue>) -> Result<Self, TooMany> {
        let fits = |(id, value): (&OptionId, &ChoiceValue)| {
            id.as_str().len() <= MAX_CHOICE_TEXT
                && match value {
                    ChoiceValue::Select { value, .. } => value.as_str().len() <= MAX_CHOICE_TEXT,
                    ChoiceValue::Toggle { .. } => true,
                }
        };
        if choices.len() <= MAX_CHOICES && choices.iter().all(fits) {
            Ok(Self(choices))
        } else {
            Err(TooMany { max: MAX_CHOICES })
        }
    }
}

impl From<Choices> for BTreeMap<OptionId, ChoiceValue> {
    fn from(choices: Choices) -> Self {
        choices.0
    }
}
