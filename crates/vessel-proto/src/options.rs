//! What a session runs with (protocol 9, milestone 10, ADR 0022): the
//! agent's own session config options (its model, its effort, its
//! permissions mode, and whatever else it offers), what a client asks for,
//! and the calls that read and change them.
//!
//! [Decision Log]
//! - Purpose: a person starting or watching a session needs to choose a
//!   model, an effort and a permissions mode, and to see what the agent is
//!   running with now. Protocol 8 carried a model name at creation and
//!   nothing else; whether the agent took it was visible only as a failure.
//! - Constraint: the choices are the agent's, not Vessel's. Agents differ
//!   in what they offer (one has an effort, another has none; model names
//!   change with every release), and ACP already reports them, with ids the
//!   agent expects back (`session/new`'s `configOptions`, ADR 0003).
//!   Vessel must pass them on truthfully: what an agent does not offer is
//!   not offered, and no list of models, efforts or modes is written into
//!   Vessel.
//! - Alternatives: a fixed set of fields (`model`, `effort`, `permissions`)
//!   with values Vessel maps per harness (the mapping goes stale with each
//!   agent release, and invents an effort for agents that have none);
//!   forwarding ACP's option objects as raw JSON (a client could not match
//!   on a kind or a category, which AGENTS.md section 2 forbids, and an ACP
//!   version bump would change Vessel's wire).
//! - Choice: Vessel's own shapes of ACP's options. A [`SessionOption`] is
//!   an id and a name the agent chose, a closed [`OptionCategory`] (ACP's
//!   five, an unknown one read as `other`), and a closed [`OptionKind`]: a
//!   select with its values or a toggle. Ids are opaque ([`OptionId`],
//!   [`OptionValueId`]), echoed back as received. A client asks for a value
//!   with an [`OptionChoice`], at creation
//!   (`SessionMode::Structured.options`) or on a running session
//!   (`session.set_option`).
//! - Grouped values (ACP's `SessionConfigSelectGroup`) are flattened in
//!   order, each value carrying its group's name: a client draws a header
//!   where the name changes, and nothing needs the group's own id.
//! - Must stay true: these are never invented by the node. An option a
//!   client sees came from the agent, in the agent's words; an option the
//!   agent stops reporting disappears from the next list.
//!
//! [`OptionId`]: crate::OptionId
//! [`OptionValueId`]: crate::OptionValueId

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agents::HarnessRef;
use crate::id::{OptionId, OptionValueId, SessionId};
use crate::name::ProfileName;

/// What an option is about, as the agent labels it. For placing a control
/// (the composer's model, effort and permissions), never for deciding what
/// a value means: the values are the agent's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum OptionCategory {
    /// The session's mode: what the agent may do without asking (the page
    /// calls it permissions).
    Mode,
    /// The model.
    Model,
    /// A setting of the model other than its effort.
    ModelConfig,
    /// How hard the model thinks (the page calls it effort).
    ThoughtLevel,
    /// Anything else, an option without a category, and a category this
    /// build does not know (ACP's custom `_` ones, or one added later):
    /// a newer node's category reads as this rather than failing the
    /// whole list.
    #[serde(other)]
    Other,
}

/// One value a select option offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OptionValue {
    /// The value's id, sent back in an [`OptionChoice`].
    pub id: OptionValueId,
    /// Its label, in the agent's words.
    pub name: String,
    /// A longer explanation, when the agent gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The name of the group the agent files it under, when it groups its
    /// values. Values of one group are adjacent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// How much a mode lets the agent do unasked (protocol 22, ADR 0058):
    /// Paseo's tiers, for a mode Vessel knows; absent for any other value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<Risk>,
    /// Whether this mode runs every action without asking (protocol 22, ADR
    /// 0058): a client names it "Full access" wherever it offers it and
    /// where a session starts in it. Left out when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unattended: bool,
}

/// How much a mode lets an agent do without asking: Paseo's colour tiers
/// (`AgentModeColorTier`), which a client draws as it draws Paseo's.
///
/// [Decision Log]
/// - Purpose: the owner asked for every agent set up as Paseo sets it up
///   (ADR 0058), and the coordinator for a mode that runs without asking to
///   be named for what it is. The agent's own label ("Bypass", "Allow all")
///   does not say that in one word, and a client cannot tell from an id.
/// - Choice: the node tags the modes it knows, from each harness's
///   descriptor (Paseo's manifest), or a direct driver sets them; a value
///   it does not know carries neither, and a client shows it plain.
/// - Forward: a tier a newer node adds reads as [`Risk::Unknown`] rather
///   than failing the whole list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    /// Asks before acting (Claude Code's `default`, Muse's `onRequest`).
    Safe,
    /// Acts on some things unasked (accepting edits, an auto reviewer).
    Moderate,
    /// Acts on everything unasked.
    Dangerous,
    /// Plans without acting.
    Planning,
    /// A tier this build does not know.
    #[serde(other)]
    Unknown,
}

/// What kind of control an option is, and its current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OptionKind {
    /// One value out of a list.
    Select {
        /// The value in effect.
        current: OptionValueId,
        /// Every value offered, in the agent's order.
        values: Vec<OptionValue>,
    },
    /// On or off.
    Toggle {
        /// Whether it is on.
        on: bool,
    },
    /// A kind this build does not know, sent by a newer node. A client
    /// skips the option: it can neither show nor set it. Without this, one
    /// option of a later kind would fail the decode of the whole list.
    #[serde(other)]
    Unknown,
}

/// One of a session's config options, as the agent reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SessionOption {
    /// The option's id, sent back in an [`OptionChoice`].
    pub id: OptionId,
    /// Its label, in the agent's words.
    pub name: String,
    /// A longer explanation, when the agent gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// What it is about.
    pub category: OptionCategory,
    /// Its control and current state.
    #[serde(flatten)]
    pub kind: OptionKind,
}

impl SessionOption {
    /// The name of its value now: the select's value as the agent names it
    /// (its id when the agent does not list it), `on` or `off` for a
    /// toggle; `None` for an option of an unknown kind.
    #[must_use]
    pub fn current_name(&self) -> Option<&str> {
        match &self.kind {
            OptionKind::Select { current, values } => Some(
                values
                    .iter()
                    .find(|value| value.id == *current)
                    .map_or_else(|| current.as_str(), |value| value.name.as_str()),
            ),
            OptionKind::Toggle { on: true } => Some("on"),
            OptionKind::Toggle { on: false } => Some("off"),
            OptionKind::Unknown => None,
        }
    }

    /// The choice that keeps it as it is now: its select's current value or
    /// its toggle's state; `None` for an unknown kind. What a session
    /// carries to where it continues (a move, an import), so it runs there
    /// as it ran here.
    #[must_use]
    pub fn choice(&self) -> Option<OptionChoice> {
        let value = match &self.kind {
            OptionKind::Select { current, .. } => ChoiceValue::Select {
                value: current.clone(),
            },
            OptionKind::Toggle { on } => ChoiceValue::Toggle { on: *on },
            OptionKind::Unknown => return None,
        };
        Some(OptionChoice {
            id: self.id.clone(),
            value,
        })
    }

    /// `Model Haiku, Mode plan, Fast mode off`: each option's name and its
    /// value's, for a line of text (an older client's stand-in, the CLI);
    /// `none` without any. Options of an unknown kind are left out.
    #[must_use]
    pub fn summary(options: &[Self]) -> String {
        let mut text = String::new();
        for option in options {
            let Some(value) = option.current_name() else {
                continue;
            };
            if !text.is_empty() {
                text.push_str(", ");
            }
            text.push_str(&option.name);
            text.push(' ');
            text.push_str(value);
        }
        if text.is_empty() {
            text.push_str("none");
        }
        text
    }
}

/// The value a client asks an option to take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChoiceValue {
    /// For a select option: one of its values.
    Select {
        /// The value.
        value: OptionValueId,
    },
    /// For a toggle: on or off.
    Toggle {
        /// Whether to turn it on.
        on: bool,
    },
}

/// A client's choice for one option: at creation
/// (`SessionMode::Structured.options`) or with `session.set_option`. A
/// choice the agent does not offer (an unknown option, a value not in the
/// list, a toggle for a select) fails, at creation the whole session, as an
/// unknown model does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OptionChoice {
    /// Which option.
    pub id: OptionId,
    /// The value it should take.
    #[serde(flatten)]
    pub value: ChoiceValue,
}

/// `harness.options` (protocol 9): what a harness's agent offers before
/// any session of it exists, for a new-session form. The node starts the
/// agent as a structured session would (under `profile`, in a scratch
/// directory), reads its options from the handshake, and ends it; no
/// prompt is sent. Needs `read`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetHarnessOptions {
    /// Which harness; an added agent needs protocol 11.
    pub harness: HarnessRef,
    /// Under this profile; `None` is the harness's default. A profile can
    /// change what an agent offers (another account, another provider).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
}

/// Reply to `harness.options`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HarnessOptions {
    /// What the agent offered, in its order. Empty for an agent that
    /// offers none.
    pub options: Vec<SessionOption>,
    /// When the node asked the agent: the node answers from a recent probe
    /// rather than start the agent for every caller.
    pub probed_at: DateTime<Utc>,
    /// What the agent said it can do (protocol 10): whether a new
    /// session's composer offers steering and images. Absent from older
    /// nodes, and for a harness without structured sessions here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abilities: Option<crate::conversation::AgentAbilities>,
}

/// `session.options` (protocol 9): a structured session's options as its
/// agent last reported them. Needs `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetSessionOptions {
    /// Which session.
    pub session: SessionId,
}

/// `session.set_option` (protocol 9): change one option of a running
/// structured session. The reply is every option as the agent reports them
/// after the change (one change can move others); the agent's refusal is
/// the call's error. Needs `operate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetSessionOption {
    /// Which session.
    pub session: SessionId,
    /// The change.
    pub choice: OptionChoice,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> SessionOption {
        SessionOption {
            id: OptionId::new("model"),
            name: "Model".to_owned(),
            description: None,
            category: OptionCategory::Model,
            kind: OptionKind::Select {
                current: OptionValueId::new("default"),
                values: vec![
                    OptionValue {
                        id: OptionValueId::new("default"),
                        name: "Default".to_owned(),
                        description: Some("The agent's choice".to_owned()),
                        group: None,
                        risk: None,
                        unattended: false,
                    },
                    OptionValue {
                        id: OptionValueId::new("haiku"),
                        name: "Haiku".to_owned(),
                        description: None,
                        group: Some("Fast".to_owned()),
                        risk: None,
                        unattended: false,
                    },
                ],
            },
        }
    }

    #[test]
    fn a_select_option_is_flat_and_tagged_by_kind() {
        let option = model();
        let text = serde_json::to_string(&option).unwrap();
        assert_eq!(
            text,
            r#"{"id":"model","name":"Model","category":"model","kind":"select","current":"default","values":[{"id":"default","name":"Default","description":"The agent's choice"},{"id":"haiku","name":"Haiku","group":"Fast"}]}"#
        );
        assert_eq!(
            serde_json::from_str::<SessionOption>(&text).unwrap(),
            option
        );
    }

    #[test]
    fn a_toggle_is_on_or_off_and_absent_fields_stay_absent() {
        let option = SessionOption {
            id: OptionId::new("fast"),
            name: "Fast mode".to_owned(),
            description: None,
            category: OptionCategory::ModelConfig,
            kind: OptionKind::Toggle { on: true },
        };
        let text = serde_json::to_string(&option).unwrap();
        assert_eq!(
            text,
            r#"{"id":"fast","name":"Fast mode","category":"model_config","kind":"toggle","on":true}"#
        );
        assert_eq!(
            serde_json::from_str::<SessionOption>(&text).unwrap(),
            option
        );
    }

    /// A later kind decodes as `unknown`, through the flattened tag, so
    /// the rest of the list still reads; it has no value and no choice.
    #[test]
    fn an_option_of_a_later_kind_decodes_as_unknown() {
        let text = r#"[{"id":"temperature","name":"Temperature","category":"model_config","kind":"slider","min":0,"max":2,"current":1},
                       {"id":"fast","name":"Fast","category":"other","kind":"toggle","on":true}]"#;
        let options: Vec<SessionOption> = serde_json::from_str(text).unwrap();
        assert_eq!(options.len(), 2);
        let later = options.first().unwrap();
        assert_eq!(later.kind, OptionKind::Unknown);
        assert_eq!(later.id, OptionId::new("temperature"));
        assert_eq!(later.current_name(), None);
        assert_eq!(later.choice(), None);
        assert_eq!(SessionOption::summary(&options), "Fast on");
    }

    #[test]
    fn an_option_keeps_its_value_as_a_choice() {
        assert_eq!(
            model().choice(),
            Some(OptionChoice {
                id: OptionId::new("model"),
                value: ChoiceValue::Select {
                    value: OptionValueId::new("default"),
                },
            })
        );
        assert_eq!(model().current_name(), Some("Default"));
        assert_eq!(SessionOption::summary(&[]), "none");
    }

    #[test]
    fn categories_are_snake_case_and_unknown_ones_read_as_other() {
        let names = [
            (OptionCategory::Mode, "\"mode\""),
            (OptionCategory::Model, "\"model\""),
            (OptionCategory::ModelConfig, "\"model_config\""),
            (OptionCategory::ThoughtLevel, "\"thought_level\""),
            (OptionCategory::Other, "\"other\""),
        ];
        for (category, text) in names {
            assert_eq!(serde_json::to_string(&category).unwrap(), text);
            assert_eq!(
                serde_json::from_str::<OptionCategory>(text).unwrap(),
                category
            );
        }
        for unknown in ["\"_vendor\"", "\"sandbox\""] {
            assert_eq!(
                serde_json::from_str::<OptionCategory>(unknown).unwrap(),
                OptionCategory::Other
            );
        }
    }

    #[test]
    fn a_choice_names_the_option_and_its_value() {
        let select = OptionChoice {
            id: OptionId::new("model"),
            value: ChoiceValue::Select {
                value: OptionValueId::new("haiku"),
            },
        };
        let text = serde_json::to_string(&select).unwrap();
        assert_eq!(text, r#"{"id":"model","kind":"select","value":"haiku"}"#);
        assert_eq!(serde_json::from_str::<OptionChoice>(&text).unwrap(), select);

        let toggle = OptionChoice {
            id: OptionId::new("fast"),
            value: ChoiceValue::Toggle { on: false },
        };
        let text = serde_json::to_string(&toggle).unwrap();
        assert_eq!(text, r#"{"id":"fast","kind":"toggle","on":false}"#);
        assert_eq!(serde_json::from_str::<OptionChoice>(&text).unwrap(), toggle);
    }

    #[test]
    fn the_requests_read_as_their_wire_form() {
        let probe = GetHarnessOptions {
            harness: crate::HarnessKind::ClaudeCode.into(),
            profile: None,
        };
        assert_eq!(
            serde_json::to_string(&probe).unwrap(),
            r#"{"harness":"claude-code"}"#
        );
        let probed = HarnessOptions {
            options: vec![model()],
            probed_at: DateTime::UNIX_EPOCH,
            abilities: None,
        };
        let text = serde_json::to_string(&probed).unwrap();
        assert!(text.starts_with(r#"{"options":[{"id":"model""#), "{text}");
        assert!(
            text.ends_with(r#""probed_at":"1970-01-01T00:00:00Z"}"#),
            "{text}"
        );
        assert_eq!(
            serde_json::from_str::<HarnessOptions>(&text).unwrap(),
            probed
        );
    }
}
