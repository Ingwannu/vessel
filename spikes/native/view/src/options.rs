//! What an agent offers (protocol 9, ADR 0022), as the controls a
//! composer shows, and the choices made before a session exists: the
//! page's `web/src/data/options.ts` and `web/src/data/picks.ts`, ported.
//!
//! [Decision Log]
//! - Purpose: the new chat chooses the agent's model, effort and
//!   permissions mode before Send (both references the owner named have
//!   these pickers on the composer; the page has them).
//! - Constraint: the options are the agent's, in its words, and change
//!   with every release (ADR 0022): nothing here lists models or modes.
//!   The page decides where each option goes; every surface must decide it
//!   the same way (ADR 0036).
//! - Alternatives: a layout of the app's own (two surfaces would then show
//!   one agent's options differently); calling the page's TypeScript (no
//!   runtime for it in the app).
//! - Choice: the page's rules, ported line for line, with its test cases
//!   (`tests/options.rs`): the first select of category `model` is the
//!   model, of `thought_level` the effort, of `mode` the permissions;
//!   every toggle is a switch; every other select is listed after them;
//!   an option of a kind this build does not know is skipped. A pick is
//!   sent only when the agent offers it and it changes something, in the
//!   agent's order (a choice the agent does not offer fails the session).
//! - Different from the page: [`own_name`] compares with the page's row
//!   names in the language shown (the page has English only), so an agent's
//!   option named "Agent" is said "Claude Code agent" in English and kept
//!   as "Agent" beside a Korean "에이전트" row, which cannot be confused
//!   with it.

use vessel_proto::{
    ChoiceValue, OptionCategory, OptionChoice, OptionId, OptionKind, OptionValue, OptionValueId,
    SessionOption,
};

use crate::Lang;

/// A select, as a composer shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectControl {
    /// The option's id, sent back in a choice.
    pub id: OptionId,
    /// Its name as shown.
    pub name: String,
    /// The agent's longer explanation.
    pub description: Option<String>,
    /// The value in effect: one of `values`, or one the agent set that it
    /// does not list.
    pub current: OptionValueId,
    /// Every value offered, in the agent's order.
    pub values: Vec<OptionValue>,
}

impl SelectControl {
    /// The value in effect, when the agent lists it.
    #[must_use]
    pub fn current_value(&self) -> Option<&OptionValue> {
        self.values.iter().find(|value| value.id == self.current)
    }

    /// What the value in effect is called (its id when not listed).
    #[must_use]
    pub fn current_name(&self) -> &str {
        self.current_value()
            .map_or_else(|| self.current.as_str(), |value| value.name.as_str())
    }

    /// The choice of `value` for this select.
    #[must_use]
    pub fn choose(&self, value: OptionValueId) -> OptionChoice {
        OptionChoice {
            id: self.id.clone(),
            value: ChoiceValue::Select { value },
        }
    }
}

/// An on/off option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleControl {
    /// The option's id.
    pub id: OptionId,
    /// Its name as shown.
    pub name: String,
    /// The agent's longer explanation.
    pub description: Option<String>,
    /// Whether it is on.
    pub on: bool,
}

impl ToggleControl {
    /// The choice that turns it `on` or off.
    #[must_use]
    pub fn choose(&self, on: bool) -> OptionChoice {
        OptionChoice {
            id: self.id.clone(),
            value: ChoiceValue::Toggle { on },
        }
    }
}

/// Everything an agent offers, by where a composer shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Controls {
    /// The model.
    pub model: Option<SelectControl>,
    /// The effort.
    pub effort: Option<SelectControl>,
    /// The permissions mode.
    pub permissions: Option<SelectControl>,
    /// Switches, under the model and effort.
    pub toggles: Vec<ToggleControl>,
    /// Other choices, after the switches.
    pub selects: Vec<SelectControl>,
    /// The one effort control (`crate::effort`): the levels, the top step
    /// and fast, when the agent sends levels. The options it drives are
    /// still listed above; a composer that shows it leaves them out of
    /// its other controls (`Effort::drives`).
    pub slider: Option<crate::effort::Effort>,
}

impl Controls {
    /// Whether there is anything to choose.
    #[must_use]
    pub fn offers_anything(&self) -> bool {
        self.model.is_some()
            || self.effort.is_some()
            || self.permissions.is_some()
            || !self.toggles.is_empty()
            || !self.selects.is_empty()
    }

    /// What the agent, model and effort control reads when closed: the
    /// model's name (the agent's when it offers no model), then the
    /// effort's.
    #[must_use]
    pub fn picker_label<'a>(&'a self, agent: &'a str) -> (&'a str, Option<&'a str>) {
        (
            self.model
                .as_ref()
                .map_or(agent, SelectControl::current_name),
            self.effort.as_ref().map(SelectControl::current_name),
        )
    }
}

/// Where one select goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Model,
    Effort,
    Permissions,
    Other,
}

const fn slot_of(category: OptionCategory) -> Slot {
    match category {
        OptionCategory::Model => Slot::Model,
        OptionCategory::ThoughtLevel => Slot::Effort,
        OptionCategory::Mode => Slot::Permissions,
        OptionCategory::ModelConfig | OptionCategory::Other => Slot::Other,
    }
}

/// What a composer calls an option the agent named `name`: the agent's own
/// word, unless the composer already uses that word for a row of its own
/// (in `lang`), when it says whose the option is ("Claude Code agent").
#[must_use]
pub fn own_name(name: &str, agent: &str, lang: Lang) -> String {
    let said = name.trim().to_lowercase();
    let words = lang.options();
    let mine = [words.agent, words.model, words.effort, words.permissions];
    if mine.iter().any(|word| word.to_lowercase() == said) {
        words.whose(agent, &said)
    } else {
        name.to_owned()
    }
}

/// `options` laid out as a composer's controls; `agent` is what the
/// composer calls the agent offering them.
#[must_use]
pub fn controls_of(options: &[SessionOption], agent: &str, lang: Lang) -> Controls {
    let mut out = Controls::default();
    for option in options {
        match &option.kind {
            OptionKind::Unknown => {}
            OptionKind::Toggle { on, .. } => out.toggles.push(ToggleControl {
                id: option.id.clone(),
                name: own_name(&option.name, agent, lang),
                description: option.description.clone(),
                on: *on,
            }),
            OptionKind::Select {
                current, values, ..
            } => {
                let control = SelectControl {
                    id: option.id.clone(),
                    name: option.name.clone(),
                    description: option.description.clone(),
                    current: current.clone(),
                    values: values.clone(),
                };
                // A named slot is called by the composer's word, so its own
                // name is not shown; every other select's is, and must not
                // repeat one.
                let named = |control: SelectControl| SelectControl {
                    name: own_name(&control.name, agent, lang),
                    ..control
                };
                let place = match slot_of(option.category) {
                    Slot::Model => &mut out.model,
                    Slot::Effort => &mut out.effort,
                    Slot::Permissions => &mut out.permissions,
                    Slot::Other => {
                        out.selects.push(named(control));
                        continue;
                    }
                };
                if place.is_none() {
                    *place = Some(control);
                } else {
                    out.selects.push(named(control));
                }
            }
        }
    }
    out
}

/// [`controls_of`], with the effort control for an agent of `kind`
/// (`None`: one added on the node).
#[must_use]
pub fn controls_with_effort(
    options: &[SessionOption],
    kind: Option<vessel_proto::HarnessKind>,
    agent: &str,
    lang: Lang,
) -> Controls {
    Controls {
        slider: crate::effort::Effort::of(kind, options),
        ..controls_of(options, agent, lang)
    }
}

/// Choices made before a session exists, one per option, in the order
/// they were made (`choices_for` puts them in the agent's).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Picks(Vec<OptionChoice>);

impl Picks {
    /// Picks from choices kept earlier (the last start's).
    #[must_use]
    pub fn from_choices(choices: Vec<OptionChoice>) -> Self {
        let mut picks = Self::default();
        for choice in choices {
            picks.pick(choice);
        }
        picks
    }

    /// Add `choice`, replacing one for the same option.
    pub fn pick(&mut self, choice: OptionChoice) {
        self.0.retain(|earlier| earlier.id != choice.id);
        self.0.push(choice);
    }

    fn get(&self, id: &OptionId) -> Option<&OptionChoice> {
        self.0.iter().find(|choice| &choice.id == id)
    }
}

/// Whether the agent would take `choice` for `option`.
fn offered(option: &SessionOption, choice: &OptionChoice) -> bool {
    match (&option.kind, &choice.value) {
        (OptionKind::Select { values, .. }, ChoiceValue::Select { value, .. }) => {
            values.iter().any(|offered| &offered.id == value)
        }
        (OptionKind::Toggle { .. }, ChoiceValue::Toggle { .. }) => true,
        _ => false,
    }
}

/// Whether `choice` is what `option` already is.
fn already(option: &SessionOption, choice: &OptionChoice) -> bool {
    match (&option.kind, &choice.value) {
        (OptionKind::Select { current, .. }, ChoiceValue::Select { value, .. }) => current == value,
        (OptionKind::Toggle { on }, ChoiceValue::Toggle { on: wanted, .. }) => on == wanted,
        _ => false,
    }
}

/// `options` as they will be once `picks` are applied: what a composer
/// shows before the session exists. A pick the agent does not offer (made
/// for another agent) is left out.
#[must_use]
pub fn with_picks(options: &[SessionOption], picks: &Picks) -> Vec<SessionOption> {
    options
        .iter()
        .map(|option| {
            let mut option = option.clone();
            if let Some(choice) = picks
                .get(&option.id)
                .filter(|choice| offered(&option, choice))
            {
                match (&mut option.kind, &choice.value) {
                    (OptionKind::Select { current, .. }, ChoiceValue::Select { value, .. }) => {
                        value.clone_into(current);
                    }
                    (OptionKind::Toggle { on }, ChoiceValue::Toggle { on: wanted, .. }) => {
                        *on = *wanted;
                    }
                    _ => {}
                }
            }
            option
        })
        .collect()
}

/// The choices to start a session with: the picks the agent offers and
/// that change something, in the order the agent lists its options.
#[must_use]
pub fn choices_for(options: &[SessionOption], picks: &Picks) -> Vec<OptionChoice> {
    options
        .iter()
        .filter_map(|option| {
            picks
                .get(&option.id)
                .filter(|choice| offered(option, choice) && !already(option, choice))
                .cloned()
        })
        .collect()
}
