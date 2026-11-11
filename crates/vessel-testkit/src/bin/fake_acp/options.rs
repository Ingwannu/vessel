//! `fake-acp`'s session config options (protocol 9), in the shape
//! `FAKE_ACP_OPTIONS` picks:
//!
//! - unset or `config`: four config options, in this order:
//!   - `mode` (category `mode`): `default`, `plan`, `locked`; the fake
//!     refuses to be set to `locked`, though it offers it (an agent's
//!     refusal);
//!   - `model` (category `model`): `fake-small` (the default), `fake-large`;
//!   - `effort` (category `thought_level`): `low` in group `Quick`, `high`
//!     (the default) in group `Deep`;
//!   - `fast` (no category): a boolean, off.
//! - `modes`: no config options; ACP's older `modes`, `default` and `plan`,
//!   set with `session/set_mode`.
//! - `none`: neither.
//!
//! The values live as long as the process, across `session/new` and
//! `session/load`, as the model always has.

use agent_client_protocol::Error;
use agent_client_protocol::v1::schema::{
    ConfigOptionUpdate, CurrentModeUpdate, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigOptionValue, SessionConfigSelectGroup, SessionConfigSelectOption, SessionMode,
    SessionModeState, SessionUpdate,
};

pub(crate) const MODELS: [&str; 2] = ["fake-small", "fake-large"];
const MODES: [&str; 3] = ["default", "plan", "locked"];
const OLDER_MODES: [&str; 2] = ["default", "plan"];
const EFFORTS: [&str; 2] = ["low", "high"];

/// What the fake offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Config,
    Modes,
    None,
}

/// The fake's options and their values.
#[derive(Debug, Clone)]
pub(crate) struct Options {
    shape: Shape,
    pub(crate) model: String,
    mode: String,
    effort: String,
    fast: bool,
}

impl Options {
    pub(crate) fn from_env() -> Self {
        let shape = match std::env::var("FAKE_ACP_OPTIONS").as_deref() {
            Ok("modes") => Shape::Modes,
            Ok("none") => Shape::None,
            Ok(_) | Err(_) => Shape::Config,
        };
        Self {
            shape,
            model: MODELS[0].to_owned(),
            mode: "default".to_owned(),
            effort: "high".to_owned(),
            fast: false,
        }
    }

    /// What `session/new` and `session/load` answer as `configOptions`.
    pub(crate) fn config(&self) -> Option<Vec<SessionConfigOption>> {
        (self.shape == Shape::Config).then(|| self.all())
    }

    /// What they answer as `modes`.
    pub(crate) fn modes(&self) -> Option<SessionModeState> {
        (self.shape == Shape::Modes).then(|| {
            SessionModeState::new(
                self.mode.clone(),
                OLDER_MODES
                    .iter()
                    .map(|mode| SessionMode::new(*mode, title(mode)))
                    .collect(),
            )
        })
    }

    fn all(&self) -> Vec<SessionConfigOption> {
        let select = |values: &[&'static str]| -> Vec<SessionConfigSelectOption> {
            values
                .iter()
                .map(|value| SessionConfigSelectOption::new(*value, title(value)))
                .collect()
        };
        vec![
            SessionConfigOption::select("mode", "Mode", self.mode.clone(), select(&MODES))
                .category(SessionConfigOptionCategory::Mode),
            SessionConfigOption::select("model", "Model", self.model.clone(), select(&MODELS))
                .category(SessionConfigOptionCategory::Model),
            SessionConfigOption::select(
                "effort",
                "Effort",
                self.effort.clone(),
                vec![
                    SessionConfigSelectGroup::new("quick", "Quick", select(&EFFORTS[..1])),
                    SessionConfigSelectGroup::new("deep", "Deep", select(&EFFORTS[1..])),
                ],
            )
            .category(SessionConfigOptionCategory::ThoughtLevel),
            SessionConfigOption::boolean("fast", "Fast", self.fast),
        ]
    }

    /// `session/set_config_option`; answers every option after it.
    pub(crate) fn set(
        &mut self,
        id: &str,
        value: &SessionConfigOptionValue,
    ) -> Result<Vec<SessionConfigOption>, Error> {
        if self.shape != Shape::Config {
            return Err(Error::invalid_params());
        }
        match (id, value) {
            ("mode", SessionConfigOptionValue::ValueId { value }) if &*value.0 == "locked" => {
                Err(Error::invalid_params().data("the fake does not allow `locked` now"))
            }
            ("mode", SessionConfigOptionValue::ValueId { value }) if MODES.contains(&&*value.0) => {
                self.mode = value.0.to_string();
                Ok(self.all())
            }
            ("model", SessionConfigOptionValue::ValueId { value })
                if MODELS.contains(&&*value.0) =>
            {
                self.model = value.0.to_string();
                Ok(self.all())
            }
            ("effort", SessionConfigOptionValue::ValueId { value })
                if EFFORTS.contains(&&*value.0) =>
            {
                self.effort = value.0.to_string();
                Ok(self.all())
            }
            ("fast", SessionConfigOptionValue::Boolean { value }) => {
                self.fast = *value;
                Ok(self.all())
            }
            _ => Err(Error::invalid_params()),
        }
    }

    /// `session/set_mode`.
    pub(crate) fn set_mode(&mut self, mode: &str) -> Result<(), Error> {
        if self.shape != Shape::Modes || !OLDER_MODES.contains(&mode) {
            return Err(Error::invalid_params());
        }
        mode.clone_into(&mut self.mode);
        Ok(())
    }

    /// The agent changes its own mode (prompt `switch <mode>`), and the
    /// update that reports it.
    pub(crate) fn switch(&mut self, mode: &str) -> Option<SessionUpdate> {
        match self.shape {
            Shape::Config if MODES.contains(&mode) => {
                mode.clone_into(&mut self.mode);
                Some(SessionUpdate::ConfigOptionUpdate(ConfigOptionUpdate::new(
                    self.all(),
                )))
            }
            Shape::Modes if OLDER_MODES.contains(&mode) => {
                mode.clone_into(&mut self.mode);
                Some(SessionUpdate::CurrentModeUpdate(CurrentModeUpdate::new(
                    mode.to_owned(),
                )))
            }
            Shape::Config | Shape::Modes | Shape::None => None,
        }
    }

    /// Prompt `options`: the values the fake runs with.
    pub(crate) fn describe(&self) -> String {
        format!(
            "options mode={} model={} effort={} fast={}",
            self.mode, self.model, self.effort, self.fast
        )
    }
}

/// `plan` → `Plan`: the fake's names for its values.
fn title(value: &str) -> String {
    let mut chars = value.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}
