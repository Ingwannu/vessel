//! The effort control: one slider for an agent's thinking levels, whose
//! last step holds the agent's own "ultra" switch, and a fast switch
//! beside it (the owner, 2026-11-13, on 0.0.0.0.7's row of named
//! controls: put Ultracode or a Max into the effort, make fast mode part
//! of the same control, and drop the separate controls).
//!
//! [Decision Log]
//! - Purpose: which of an agent's options the effort control drives, so
//!   the composer shows one control for "how hard and how fast" instead of
//!   a control per option.
//! - Constraint: the options are the agent's, in its words, and change
//!   with every release (ADR 0022): the levels are always the agent's
//!   `thought_level` select, in its order; only the two switches folded
//!   into the control (the top step, fast) have to be known by id.
//! - Alternatives: matching the switches by name ("Fast mode",
//!   "Ultracode": the agent's words, translated or renamed by a release);
//!   a rule over categories (Claude Code's fast mode is `model_config`, as
//!   are options that are neither).
//! - Choice: a typed table, [`ROLES`], of the options the control folds in
//!   by their role (the top step's switch, fast), each row one agent kind
//!   and the ids its adapters use, each option either a toggle or a select
//!   of an on and an off value (Claude Code has shipped both shapes). A
//!   third agent, or a new adapter's id, is one row (the coordinator,
//!   2026-11-14: Codex-only options such as a multi-tier speed may come;
//!   the controls are written against the role, not today's ids). An
//!   agent the table does not know gets its levels only; an option the
//!   table names that the agent does not send is ignored.
//! - The top stop: the purple state (the Codex app's "maximum") belongs to
//!   the last step when it is the folded top switch or a level the table
//!   [`MAXIMUM_LEVELS`] names (Codex's `ultra`, Claude Code's `max`), never
//!   to a word in the drawing code ([`Effort::maximum`]).
//! - The levels leave out "default": the control's reset is the default.

use vessel_proto::{
    ChoiceValue, HarnessKind, OptionCategory, OptionChoice, OptionId, OptionKind, OptionValueId,
    SessionOption,
};

/// What an option folded into the control does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The step after the highest level ("ultra").
    Top,
    /// Fast mode.
    Fast,
}

/// One row of [`ROLES`]: an agent kind's option of a role, by the ids its
/// adapters have used (one per release shape, first match wins).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Known {
    /// The agent.
    pub kind: HarnessKind,
    /// What the option does.
    pub role: Role,
    /// Its ids.
    pub ids: &'static [&'static str],
}

/// The options the control knows, by the ids the adapters send (read off
/// the preview node, 2026-11-13, through the app's log of each probe):
/// Claude Code's `ultracode` and `fast` (on|off selects; its older
/// adapter's `fast_mode` toggle), Codex's `fast-mode` (an off|on select).
/// Codex's top levels (Max, Ultra) are levels of its own.
pub const ROLES: &[Known] = &[
    Known {
        kind: HarnessKind::ClaudeCode,
        role: Role::Top,
        ids: &["ultracode"],
    },
    Known {
        kind: HarnessKind::ClaudeCode,
        role: Role::Fast,
        ids: &["fast", "fast_mode"],
    },
    Known {
        kind: HarnessKind::Codex,
        role: Role::Fast,
        ids: &["fast-mode"],
    },
];

/// The level values that are an agent's maximum when they are its last
/// step: Codex's `ultra`, Claude Code's `max`.
pub const MAXIMUM_LEVELS: &[&str] = &["ultra", "max"];

/// A switch found among the agent's options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Switch {
    /// An on/off option.
    Toggle {
        /// Its id.
        id: OptionId,
        /// Whether it is on.
        on: bool,
    },
    /// A select of two values, one of them "on".
    Select {
        /// Its id.
        id: OptionId,
        /// The value that turns it on.
        on_value: OptionValueId,
        /// The value that turns it off.
        off_value: OptionValueId,
        /// Whether the value in effect is the on one.
        on: bool,
    },
}

impl Switch {
    /// The option's id.
    #[must_use]
    pub const fn id(&self) -> &OptionId {
        match self {
            Self::Toggle { id, .. } | Self::Select { id, .. } => id,
        }
    }

    /// Whether it is on.
    #[must_use]
    pub const fn on(&self) -> bool {
        match self {
            Self::Toggle { on, .. } | Self::Select { on, .. } => *on,
        }
    }

    /// The choice that turns it `on` or off.
    #[must_use]
    pub fn choose(&self, on: bool) -> OptionChoice {
        match self {
            Self::Toggle { id, .. } => OptionChoice {
                id: id.clone(),
                value: ChoiceValue::Toggle { on },
            },
            Self::Select {
                id,
                on_value,
                off_value,
                ..
            } => OptionChoice {
                id: id.clone(),
                value: ChoiceValue::Select {
                    value: if on {
                        on_value.clone()
                    } else {
                        off_value.clone()
                    },
                },
            },
        }
    }

    /// `option` as a switch: a toggle, or a select whose values include
    /// `on` and `off` (by id, as the agents send them).
    fn of(option: &SessionOption) -> Option<Self> {
        match &option.kind {
            OptionKind::Toggle { on, .. } => Some(Self::Toggle {
                id: option.id.clone(),
                on: *on,
            }),
            OptionKind::Select {
                current, values, ..
            } => {
                let find = |word: &str| {
                    values
                        .iter()
                        .find(|value| value.id.as_str().eq_ignore_ascii_case(word))
                        .map(|value| value.id.clone())
                };
                let on_value = find("on")?;
                let off_value = find("off")?;
                Some(Self::Select {
                    id: option.id.clone(),
                    on: *current == on_value,
                    on_value,
                    off_value,
                })
            }
            OptionKind::Unknown => None,
        }
    }
}

/// One step of the slider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// One of the agent's levels.
    Level {
        /// Its value's id.
        value: OptionValueId,
        /// Its name, as the agent says it.
        name: String,
    },
    /// The top switch on, at the highest level.
    Top {
        /// The switch's name, as the agent says it.
        name: String,
    },
}

/// The effort control for one agent's options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effort {
    /// The levels' option's id.
    pub level_id: OptionId,
    /// The slider's steps, lowest first.
    pub steps: Vec<Step>,
    /// The step in effect; `None` when the level in effect is the agent's
    /// "default" or one it does not list.
    pub current: Option<usize>,
    /// The top step's switch, when the agent has one the table knows.
    pub top: Option<Switch>,
    /// The fast switch, when the agent has one the table knows.
    pub fast: Option<Switch>,
}

/// The value id every agent so far uses for "the agent's own level".
const DEFAULT: &str = "default";

impl Effort {
    /// The control for `options` from an agent of `kind` (`None`: one
    /// added on the node, which the table cannot know); `None` when it
    /// sends no levels.
    #[must_use]
    pub fn of(kind: Option<HarnessKind>, options: &[SessionOption]) -> Option<Self> {
        let (level_id, current_value, values) =
            options
                .iter()
                .find_map(|option| match (&option.kind, option.category) {
                    (OptionKind::Select { current, values }, OptionCategory::ThoughtLevel) => {
                        Some((option.id.clone(), current.clone(), values))
                    }
                    _ => None,
                })?;
        let mut steps: Vec<Step> = values
            .iter()
            .filter(|value| value.id.as_str() != DEFAULT)
            .map(|value| Step::Level {
                value: value.id.clone(),
                name: value.name.clone(),
            })
            .collect();
        let find = |role: Role| {
            let kind = kind?;
            ROLES
                .iter()
                .filter(|known| known.kind == kind && known.role == role)
                .flat_map(|known| known.ids.iter())
                .find_map(|id| {
                    let option = options.iter().find(|option| option.id.as_str() == *id)?;
                    Switch::of(option).map(|switch| (option.name.clone(), switch))
                })
        };
        let top = find(Role::Top);
        let fast = find(Role::Fast).map(|(_, switch)| switch);
        let top_on = top.as_ref().is_some_and(|(_, switch)| switch.on());
        if let Some((name, _)) = &top {
            steps.push(Step::Top { name: name.clone() });
        }
        let level = steps.iter().position(|step| match step {
            Step::Level { value, .. } => *value == current_value,
            Step::Top { .. } => false,
        });
        let current = if top_on && level.is_some() {
            steps.len().checked_sub(1)
        } else {
            level
        };
        Some(Self {
            level_id,
            steps,
            current,
            top: top.map(|(_, switch)| switch),
            fast,
        })
    }

    /// The choices that move the control to step `index`: its level (the
    /// highest for the top step), and the top switch on at the top step,
    /// off below it (only when it changes).
    #[must_use]
    pub fn choose(&self, index: usize) -> Vec<OptionChoice> {
        let Some(step) = self.steps.get(index) else {
            return Vec::new();
        };
        let level = match step {
            Step::Level { value, .. } => Some(value.clone()),
            Step::Top { .. } => self.steps.iter().rev().find_map(|step| match step {
                Step::Level { value, .. } => Some(value.clone()),
                Step::Top { .. } => None,
            }),
        };
        let mut choices: Vec<OptionChoice> = level
            .map(|value| OptionChoice {
                id: self.level_id.clone(),
                value: ChoiceValue::Select { value },
            })
            .into_iter()
            .collect();
        if let Some(top) = &self.top {
            let on = matches!(step, Step::Top { .. });
            if top.on() != on {
                choices.push(top.choose(on));
            }
        }
        choices
    }

    /// The choices that put the control back to the agent's own: its
    /// "default" level when it lists one, and the top switch off.
    #[must_use]
    pub fn reset(&self) -> Vec<OptionChoice> {
        let mut choices = vec![OptionChoice {
            id: self.level_id.clone(),
            value: ChoiceValue::Select {
                value: OptionValueId::new(DEFAULT),
            },
        }];
        if let Some(top) = self.top.as_ref().filter(|top| top.on()) {
            choices.push(top.choose(false));
        }
        choices
    }

    /// The top stop, drawn in the maximum's colours: the last step, when
    /// it is the folded top switch or a level [`MAXIMUM_LEVELS`] names.
    #[must_use]
    pub fn maximum(&self) -> Option<usize> {
        let last = self.steps.len().checked_sub(1)?;
        let top = match self.steps.get(last)? {
            Step::Top { .. } => true,
            Step::Level { value, .. } => MAXIMUM_LEVELS.contains(&value.as_str()),
        };
        top.then_some(last)
    }

    /// Whether `id` is one of the options this control drives (the
    /// composer lists the others elsewhere).
    #[must_use]
    pub fn drives(&self, id: &OptionId) -> bool {
        *id == self.level_id
            || self.top.as_ref().is_some_and(|top| top.id() == id)
            || self.fast.as_ref().is_some_and(|fast| fast.id() == id)
    }
}

#[cfg(test)]
mod tests;
