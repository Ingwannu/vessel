//! Choosing the requested model among an agent's session config options.
//!
//! [Decision Log]
//! - Purpose: `SessionMode::Structured { model }` must select a model for
//!   any harness.
//! - Alternatives: per-harness CLI flags or environment variables (they
//!   differ per harness, and `opencode acp` has none); ACP's unstable
//!   `session/set_model` (behind an unstable crate feature that may change
//!   in any minor release).
//! - Choice: the stable ACP session config option whose category is
//!   `model`, set with `session/set_config_option`, as Paseo does. The
//!   requested name is matched against option values exactly, then without
//!   regard to case against values and display names.
//! - Failure is loud: a model the agent does not offer fails the session
//!   instead of silently running the default (which may be a far more
//!   expensive model than the one asked for).
//! - Since protocol 9 a client may also choose the model option by its id
//!   and value (`options`); this name match stays for
//!   `SessionMode::Structured.model`, which names a model as people do.

use std::fmt;

use agent_client_protocol::v1::schema as acp;
use vessel_proto::ModelId;

use crate::options::{Change, flatten};

/// Why the requested model cannot be selected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ModelError {
    /// The agent offers no model config option.
    #[error("the agent offers no model selection; cannot select `{wanted}`")]
    NoModelOption { wanted: ModelId },
    /// The agent offers models, but not this one.
    #[error(
        "the agent does not offer model `{wanted}` (offered: {})",
        Offered(offered)
    )]
    NotOffered {
        wanted: ModelId,
        offered: Vec<acp::SessionConfigValueId>,
    },
}

/// The change needed to run `wanted`, or `None` if it is already current.
pub(crate) fn choose(
    options: &[acp::SessionConfigOption],
    wanted: &ModelId,
) -> Result<Option<Change>, ModelError> {
    let Some((config, select)) = options.iter().find_map(|option| {
        let is_model = matches!(
            option.category,
            Some(acp::SessionConfigOptionCategory::Model)
        );
        match &option.kind {
            acp::SessionConfigKind::Select(select) if is_model => Some((&option.id, select)),
            acp::SessionConfigKind::Select(_) | acp::SessionConfigKind::Boolean(_) | _ => None,
        }
    }) else {
        return Err(ModelError::NoModelOption {
            wanted: wanted.clone(),
        });
    };
    let choices: Vec<&acp::SessionConfigSelectOption> = flatten(&select.options)
        .into_iter()
        .map(|(choice, _group)| choice)
        .collect();
    let name = wanted.as_str();
    let found = choices
        .iter()
        .find(|choice| &*choice.value.0 == name)
        .or_else(|| {
            choices.iter().find(|choice| {
                choice.value.0.eq_ignore_ascii_case(name) || choice.name.eq_ignore_ascii_case(name)
            })
        });
    let Some(found) = found else {
        return Err(ModelError::NotOffered {
            wanted: wanted.clone(),
            offered: choices.iter().map(|choice| choice.value.clone()).collect(),
        });
    };
    if found.value == select.current_value {
        return Ok(None);
    }
    Ok(Some(Change::Config {
        config: config.clone(),
        value: acp::SessionConfigOptionValue::value_id(found.value.clone()),
    }))
}

/// Offered values, comma-separated, for the error message.
struct Offered<'a>(&'a [acp::SessionConfigValueId]);

impl fmt::Display for Offered<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, value) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            f.write_str(&value.0)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(config: &str, value: &str) -> Change {
        Change::Config {
            config: acp::SessionConfigId::new(config),
            value: acp::SessionConfigOptionValue::value_id(acp::SessionConfigValueId::new(value)),
        }
    }

    fn models(current: &str) -> Vec<acp::SessionConfigOption> {
        vec![
            acp::SessionConfigOption::select(
                "mode",
                "Mode",
                "default",
                vec![acp::SessionConfigSelectOption::new("default", "Default")],
            )
            .category(acp::SessionConfigOptionCategory::Mode),
            acp::SessionConfigOption::select(
                "model",
                "Model",
                current.to_owned(),
                vec![
                    acp::SessionConfigSelectOption::new("default", "Default (recommended)"),
                    acp::SessionConfigSelectOption::new("haiku", "Haiku"),
                ],
            )
            .category(acp::SessionConfigOptionCategory::Model),
        ]
    }

    #[test]
    fn picks_the_model_option_by_value_then_by_name() {
        let choice = choose(&models("default"), &ModelId::new("haiku")).unwrap();
        assert_eq!(choice, Some(set("model", "haiku")));
        let by_name = choose(&models("default"), &ModelId::new("HAIKU")).unwrap();
        assert_eq!(by_name, Some(set("model", "haiku")));
    }

    /// The model option `claude-agent-acp` 0.100.0 offered in a recorded
    /// `session/new` (2026-11-05).
    const RECORDED_CLAUDE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/acp/claude-agent-acp-0.100.0.model-option.json"
    ));

    #[test]
    fn selects_models_from_the_recorded_claude_adapter_option() {
        let options: Vec<acp::SessionConfigOption> = serde_json::from_str(RECORDED_CLAUDE).unwrap();
        for wanted in ["haiku", "claude-haiku-4-5-20251001", "Sonnet"] {
            let Some(Change::Config { config, value }) =
                choose(&options, &ModelId::new(wanted)).unwrap()
            else {
                panic!("{wanted} selects a config value");
            };
            assert_eq!(&*config.0, "model");
            let value = value.as_value_id().unwrap();
            assert!(value.0.eq_ignore_ascii_case(wanted), "{wanted}");
        }
        assert_eq!(choose(&options, &ModelId::new("default")), Ok(None));
        assert!(matches!(
            choose(&options, &ModelId::new("gpt-4")),
            Err(ModelError::NotOffered { .. })
        ));
    }

    #[test]
    fn the_current_model_needs_no_change() {
        assert_eq!(choose(&models("haiku"), &ModelId::new("haiku")), Ok(None));
    }

    #[test]
    fn unknown_models_and_missing_options_are_errors() {
        let error = choose(&models("default"), &ModelId::new("opus")).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the agent does not offer model `opus` (offered: default, haiku)"
        );
        assert!(matches!(
            choose(&[], &ModelId::new("opus")),
            Err(ModelError::NoModelOption { .. })
        ));
    }
}
