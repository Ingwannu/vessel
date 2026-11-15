//! The browser demo's fixed agent catalog and ACP option shapes. This
//! supplies display data only; the parent owns sessions and their logs.

use vessel_app_view::{Agent, agent_name};
use vessel_proto::{
    HarnessKind, HarnessRef, OptionCategory, OptionId, OptionKind, OptionValue, OptionValueId,
    SessionOption,
};

/// What every demo agent offers before a session: a model, an effort and
/// a permission mode (the new chat's pickers), as an ACP agent lists them.
pub(crate) fn options() -> Vec<SessionOption> {
    let select =
        |id: &str, name: &str, category, current: &str, values: &[(&str, &str)]| SessionOption {
            id: OptionId::new(id),
            name: name.to_owned(),
            description: None,
            category,
            kind: OptionKind::Select {
                current: OptionValueId::new(current),
                values: values
                    .iter()
                    .map(|(value, words)| OptionValue {
                        id: OptionValueId::new(*value),
                        name: (*words).to_owned(),
                        description: None,
                        group: None,
                    })
                    .collect(),
            },
        };
    vec![
        select(
            "model",
            "Model",
            OptionCategory::Model,
            "opus",
            &[
                ("opus", "Opus 5.5"),
                ("sonnet", "Sonnet 5.5"),
                ("haiku", "Haiku 5.5"),
            ],
        ),
        select(
            "effort",
            "Effort",
            OptionCategory::ThoughtLevel,
            "high",
            &[("low", "Low"), ("medium", "Medium"), ("high", "High")],
        ),
        select(
            "mode",
            "Permissions",
            OptionCategory::Mode,
            "default",
            &[
                ("default", "Ask"),
                ("acceptEdits", "Accept edits"),
                ("plan", "Plan"),
            ],
        ),
    ]
}

pub(super) fn agents() -> Vec<Agent> {
    [HarnessKind::ClaudeCode, HarnessKind::Codex]
        .into_iter()
        .map(|kind| {
            let harness = HarnessRef::Known(kind);
            Agent {
                name: agent_name(&harness),
                harness,
                unavailable: None,
                not_installed: false,
            }
        })
        .collect()
}
