//! The agent's session config options (protocol 9, ADR 0022): kept as
//! ACP reported them, shown as Vessel's [`SessionOption`]s, and changed by
//! a client's [`OptionChoice`].
//!
//! [Decision Log]
//! - Purpose: a client chooses a permissions mode, a model, an effort, and
//!   whatever else the agent offers, at creation and on a running session,
//!   and sees what the agent runs with now. Only the agent knows what it
//!   offers; Vessel passes that on and writes no list of its own.
//! - Constraint: ACP has two ways to say this. Session config options
//!   (`configOptions`, with a category each) are the current one; an older
//!   agent reports only `modes` (`SessionModeState`), changed with
//!   `session/set_mode` and reported with `current_mode_update`.
//! - Choice: [`Options`] keeps both as the agent last reported them and
//!   answers every question from them: the list a client sees, whether a
//!   choice is offered, and what request makes it. An agent with `modes`
//!   and no option in the `mode` category gets one synthesized from them,
//!   so a client sees one shape whatever the agent's age; choosing it
//!   becomes `session/set_mode`. An agent that has both reports its mode
//!   option, and the older `modes` are not shown twice.
//! - The ACP state is kept, not the Vessel list: a change is checked
//!   against what the agent offered, by its own ids, and an update replaces
//!   ACP's state, from which the list is derived again.
//! - Must stay true: nothing here invents a value. An option of a kind
//!   this build cannot show (ACP's kinds are open) is left out, and so
//!   cannot be chosen.

use std::fmt;
use std::sync::Arc;

use agent_client_protocol::v1::schema as acp;
use vessel_proto::{
    ChoiceValue, ModelId, OptionCategory, OptionChoice, OptionId, OptionKind, OptionValue,
    OptionValueId, SessionOption,
};

use crate::model::{self, ModelError};

/// Ids a synthesized mode option may take, in order: the first that no
/// config option of the agent uses.
const MODE_IDS: [&str; 2] = ["mode", "session_mode"];

/// One session's options, as the agent last reported them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Options {
    config: Vec<acp::SessionConfigOption>,
    /// ACP's older modes; shown only when no config option is a mode.
    modes: Option<acp::SessionModeState>,
}

/// A request that makes a choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Change {
    /// `session/set_config_option`.
    Config {
        config: acp::SessionConfigId,
        value: acp::SessionConfigOptionValue,
    },
    /// `session/set_mode`, for a mode option synthesized from `modes`.
    Mode(acp::SessionModeId),
}

/// What the agent's answer to a [`Change`] says about its options now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Applied {
    /// Every config option after the change (ACP's answer is the full
    /// set).
    Config(Vec<acp::SessionConfigOption>),
    /// The mode it set; `session/set_mode`'s answer carries no state.
    Mode(acp::SessionModeId),
}

/// A choice the agent does not offer. Checked against the agent's own
/// report, before anything is sent to it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OptionError {
    /// No option has this id.
    #[error("the agent offers no option `{id}` (offered: {})", List(offered))]
    Unknown {
        /// The id asked for.
        id: OptionId,
        /// The options it offers.
        offered: Vec<OptionId>,
    },
    /// The option has no such value.
    #[error("option `{id}` has no value `{value}` (offered: {})", List(offered))]
    NotOffered {
        /// The option.
        id: OptionId,
        /// The value asked for.
        value: OptionValueId,
        /// The values it offers.
        offered: Vec<OptionValueId>,
    },
    /// A toggle chosen like a select, or the other way round.
    #[error("option `{id}` is a {kind}; choose it as one")]
    WrongKind {
        /// The option.
        id: OptionId,
        /// What it is.
        kind: KindName,
    },
}

/// The kind of an option, for [`OptionError::WrongKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindName {
    /// One value out of a list.
    Select,
    /// On or off.
    Toggle,
}

impl fmt::Display for KindName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Select => "select",
            Self::Toggle => "toggle",
        })
    }
}

/// Ids, comma-separated, for error messages.
struct List<'a, T>(&'a [T]);

impl<T: fmt::Display> fmt::Display for List<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("none");
        }
        for (index, item) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            item.fmt(f)?;
        }
        Ok(())
    }
}

impl Options {
    /// What `session/new` or `session/load` answered.
    pub(crate) fn new(
        config: Option<Vec<acp::SessionConfigOption>>,
        modes: Option<acp::SessionModeState>,
    ) -> Self {
        Self {
            config: config.unwrap_or_default(),
            modes,
        }
    }

    /// The options as a client sees them: a synthesized mode first, when
    /// there is one, then the config options in the agent's order.
    pub(crate) fn listed(&self) -> Vec<SessionOption> {
        let mut listed = Vec::with_capacity(self.config.len().saturating_add(1));
        listed.extend(self.synthesized());
        listed.extend(self.config.iter().filter_map(option));
        listed
    }

    /// The agent reported its config options (`config_option_update`).
    pub(crate) fn replace(&mut self, config: Vec<acp::SessionConfigOption>) {
        self.config = config;
    }

    /// The agent reported its mode (`current_mode_update`).
    pub(crate) fn mode_is(&mut self, mode: acp::SessionModeId) {
        if let Some(modes) = &mut self.modes {
            modes.current_mode_id = mode;
        }
    }

    /// The agent answered a change.
    pub(crate) fn apply(&mut self, applied: Applied) {
        match applied {
            Applied::Config(config) => self.replace(config),
            Applied::Mode(mode) => self.mode_is(mode),
        }
    }

    /// The request that selects the model `wanted`, or `None` when it is
    /// the current one (see `model`).
    pub(crate) fn model(&self, wanted: &ModelId) -> Result<Option<Change>, ModelError> {
        model::choose(&self.config, wanted)
    }

    /// The request that makes `choice`, or `None` when the option has that
    /// value already.
    pub(crate) fn change(&self, choice: &OptionChoice) -> Result<Option<Change>, OptionError> {
        let id = &choice.id;
        if let Some(mode) = self.synthesized()
            && mode.id == *id
        {
            let ChoiceValue::Select { value } = &choice.value else {
                return Err(wrong_kind(id, KindName::Select));
            };
            let OptionKind::Select { current, values } = &mode.kind else {
                return Err(wrong_kind(id, KindName::Toggle));
            };
            check_offered(id, value, values.iter().map(|offered| offered.id.as_str()))?;
            return Ok(
                (current != value).then(|| Change::Mode(acp::SessionModeId::new(value.as_str())))
            );
        }
        let Some(option) = self
            .config
            .iter()
            .find(|option| *option.id.0 == *id.as_str())
        else {
            return Err(self.unknown(id));
        };
        match (&option.kind, &choice.value) {
            (acp::SessionConfigKind::Select(select), ChoiceValue::Select { value }) => {
                let offered = flatten(&select.options);
                check_offered(
                    id,
                    value,
                    offered.iter().map(|(offered, _group)| &*offered.value.0),
                )?;
                Ok(
                    (*select.current_value.0 != *value.as_str()).then(|| Change::Config {
                        config: option.id.clone(),
                        value: acp::SessionConfigOptionValue::value_id(
                            acp::SessionConfigValueId::new(value.as_str()),
                        ),
                    }),
                )
            }
            (acp::SessionConfigKind::Boolean(boolean), ChoiceValue::Toggle { on }) => {
                Ok((boolean.current_value != *on).then(|| Change::Config {
                    config: option.id.clone(),
                    value: acp::SessionConfigOptionValue::boolean(*on),
                }))
            }
            (acp::SessionConfigKind::Select(_), ChoiceValue::Toggle { .. }) => {
                Err(wrong_kind(id, KindName::Select))
            }
            (acp::SessionConfigKind::Boolean(_), ChoiceValue::Select { .. }) => {
                Err(wrong_kind(id, KindName::Toggle))
            }
            // A kind this build does not show is not offered.
            (_, ChoiceValue::Select { .. } | ChoiceValue::Toggle { .. }) => Err(self.unknown(id)),
        }
    }

    fn unknown(&self, id: &OptionId) -> OptionError {
        OptionError::Unknown {
            id: id.clone(),
            offered: self.listed().into_iter().map(|option| option.id).collect(),
        }
    }

    /// The mode option made from ACP's older `modes`, when the agent has
    /// them and no config option is a mode.
    fn synthesized(&self) -> Option<SessionOption> {
        let modes = self.modes.as_ref()?;
        let has_mode_option = self.config.iter().any(|option| {
            matches!(
                option.category,
                Some(acp::SessionConfigOptionCategory::Mode)
            )
        });
        if has_mode_option || modes.available_modes.is_empty() {
            return None;
        }
        let id = MODE_IDS
            .into_iter()
            .find(|candidate| !self.config.iter().any(|option| &*option.id.0 == *candidate))?;
        Some(SessionOption {
            id: OptionId::new(id),
            // ACP names the modes, not the selector; the category says what
            // it is, and this is only its label.
            name: "Mode".to_owned(),
            description: None,
            category: OptionCategory::Mode,
            kind: OptionKind::Select {
                current: OptionValueId::new(Arc::clone(&modes.current_mode_id.0)),
                values: modes
                    .available_modes
                    .iter()
                    .map(|mode| OptionValue {
                        id: OptionValueId::new(Arc::clone(&mode.id.0)),
                        name: mode.name.clone(),
                        description: mode.description.clone(),
                        group: None,
                        risk: None,
                        unattended: false,
                    })
                    .collect(),
            },
        })
    }
}

/// `value` is one of `offered`, or the error naming them.
fn check_offered<'a>(
    id: &OptionId,
    value: &OptionValueId,
    offered: impl Iterator<Item = &'a str> + Clone,
) -> Result<(), OptionError> {
    if offered.clone().any(|offered| offered == value.as_str()) {
        return Ok(());
    }
    Err(OptionError::NotOffered {
        id: id.clone(),
        value: value.clone(),
        offered: offered.map(OptionValueId::new).collect(),
    })
}

fn wrong_kind(id: &OptionId, kind: KindName) -> OptionError {
    OptionError::WrongKind {
        id: id.clone(),
        kind,
    }
}

/// One config option as a client sees it; `None` for a kind this build
/// does not know.
fn option(option: &acp::SessionConfigOption) -> Option<SessionOption> {
    let kind = match &option.kind {
        acp::SessionConfigKind::Select(select) => OptionKind::Select {
            current: OptionValueId::new(Arc::clone(&select.current_value.0)),
            values: flatten(&select.options)
                .into_iter()
                .map(|(value, group)| OptionValue {
                    id: OptionValueId::new(Arc::clone(&value.value.0)),
                    name: value.name.clone(),
                    description: value.description.clone(),
                    group: group.map(ToOwned::to_owned),
                    risk: None,
                    unattended: false,
                })
                .collect(),
        },
        acp::SessionConfigKind::Boolean(boolean) => OptionKind::Toggle {
            on: boolean.current_value,
        },
        _ => {
            tracing::debug!(option = %option.id.0, "an option of a kind this build does not know is left out");
            return None;
        }
    };
    Some(SessionOption {
        id: OptionId::new(Arc::clone(&option.id.0)),
        name: option.name.clone(),
        description: option.description.clone(),
        category: category(option.category.as_ref()),
        kind,
    })
}

/// ACP's category as Vessel's; none, a custom one and a newer one are
/// `other`.
fn category(category: Option<&acp::SessionConfigOptionCategory>) -> OptionCategory {
    match category {
        Some(acp::SessionConfigOptionCategory::Mode) => OptionCategory::Mode,
        Some(acp::SessionConfigOptionCategory::Model) => OptionCategory::Model,
        Some(acp::SessionConfigOptionCategory::ModelConfig) => OptionCategory::ModelConfig,
        Some(acp::SessionConfigOptionCategory::ThoughtLevel) => OptionCategory::ThoughtLevel,
        Some(acp::SessionConfigOptionCategory::Other(_) | _) | None => OptionCategory::Other,
    }
}

/// A select's values in order, each with its group's name when grouped.
pub(crate) fn flatten(
    options: &acp::SessionConfigSelectOptions,
) -> Vec<(&acp::SessionConfigSelectOption, Option<&str>)> {
    match options {
        acp::SessionConfigSelectOptions::Ungrouped(options) => {
            options.iter().map(|option| (option, None)).collect()
        }
        acp::SessionConfigSelectOptions::Grouped(groups) => groups
            .iter()
            .flat_map(|group| {
                group
                    .options
                    .iter()
                    .map(|option| (option, Some(group.name.as_str())))
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
