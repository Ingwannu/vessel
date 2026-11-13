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
//! - Choice: a typed table, [`KNOWN`], per agent kind: the top step's
//!   switch and the fast switch by option id, each either a toggle or a
//!   select of an on and an off value (Claude Code has shipped both
//!   shapes). An agent the table does not know gets its levels only; an
//!   option the table names that the agent does not send is ignored.
//! - The levels leave out "default": the control's reset is the default.

use vessel_proto::{
    ChoiceValue, HarnessKind, OptionCategory, OptionChoice, OptionId, OptionKind, OptionValueId,
    SessionOption,
};

/// A switch the control folds in, as the table names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Known {
    /// The option's ids the agent has used for it (one per release
    /// shape), first match wins.
    pub ids: &'static [&'static str],
}

/// What the effort control folds in for one agent kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    /// The agent.
    pub kind: HarnessKind,
    /// The switch that is the step after the highest level ("ultra").
    pub top: Option<Known>,
    /// The fast switch.
    pub fast: Option<Known>,
}

/// The agents whose switches the control knows. Codex's top levels (Max,
/// Ultra) are levels of its own; its fast switch is `fast` when it sends
/// one.
pub const KNOWN: &[Shape] = &[
    Shape {
        kind: HarnessKind::ClaudeCode,
        top: Some(Known {
            ids: &["ultracode"],
        }),
        fast: Some(Known {
            ids: &["fast", "fast_mode"],
        }),
    },
    Shape {
        kind: HarnessKind::Codex,
        top: None,
        fast: Some(Known {
            ids: &["fast", "fast_mode"],
        }),
    },
];

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
            OptionKind::Toggle { on } => Some(Self::Toggle {
                id: option.id.clone(),
                on: *on,
            }),
            OptionKind::Select { current, values } => {
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
        let shape = kind.and_then(|kind| KNOWN.iter().find(|shape| shape.kind == kind));
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
        let find = |known: Option<Known>| {
            let known = known?;
            known.ids.iter().find_map(|id| {
                let option = options.iter().find(|option| option.id.as_str() == *id)?;
                Switch::of(option).map(|switch| (option.name.clone(), switch))
            })
        };
        let top = find(shape.and_then(|shape| shape.top));
        let fast = find(shape.and_then(|shape| shape.fast)).map(|(_, switch)| switch);
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
