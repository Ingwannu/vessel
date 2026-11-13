//! `FAKE_ACP_OPTIONS=real`: config options in the shapes a real agent
//! sends, for the app's pickers to be seen with them (the owner's capture
//! of app build 0.0.0.0.5 with Claude Code: an `Agent` option whose values'
//! descriptions run to several lines ran into the rows under them, and an
//! on/off option with no name).
//!
//! What it offers, in this order:
//! - `mode` (category `mode`): five modes, each with a description;
//! - `model` (category `model`): sixteen models, descriptions of one long
//!   line, a name far longer than a list is wide, a Korean name;
//! - `effort` (category `thought_level`): five levels;
//! - `fast_mode`: on/off, with no name (as the capture showed it);
//! - `ultracode`: a select of two values;
//! - `agent`: the default and fifteen subagents, `ocx-*`, with
//!   descriptions of several lines, one Korean.
//!
//! Every value can be chosen; the fake refuses nothing here.

use agent_client_protocol::Error;
use agent_client_protocol::v1::schema::{
    SessionConfigOption, SessionConfigOptionCategory, SessionConfigOptionValue,
    SessionConfigSelectOption,
};

/// A select's id, name, category, and values (id, name, description).
type Select = (
    &'static str,
    &'static str,
    Option<Category>,
    &'static [(&'static str, &'static str, Option<&'static str>)],
);

/// The categories used here (the schema's own is not `const`).
#[derive(Debug, Clone, Copy)]
enum Category {
    Mode,
    Model,
    ThoughtLevel,
}

impl Category {
    const fn schema(self) -> SessionConfigOptionCategory {
        match self {
            Self::Mode => SessionConfigOptionCategory::Mode,
            Self::Model => SessionConfigOptionCategory::Model,
            Self::ThoughtLevel => SessionConfigOptionCategory::ThoughtLevel,
        }
    }
}

const SUBAGENT: &str = "Use this agent for wide read-only sweeps when answering means reading \
across many files and you only need the conclusion.\nIt reads excerpts rather than whole files, \
so it locates code; it does not review it.\n\nExamples:\n- find every caller of a function\n- \
list the modules that read a setting";

const MODES: Select = (
    "mode",
    "Mode",
    Some(Category::Mode),
    &[
        (
            "default",
            "Default",
            Some("Standard behavior, prompts for dangerous operations"),
        ),
        (
            "acceptEdits",
            "Accept Edits",
            Some("Auto-accept file edit operations"),
        ),
        (
            "plan",
            "Plan Mode",
            Some("Planning mode, no actual tool execution"),
        ),
        (
            "dontAsk",
            "Don't Ask",
            Some("Don't prompt for permissions, deny if not pre-approved"),
        ),
        (
            "bypassPermissions",
            "Bypass Permissions",
            Some("Bypass all permission checks"),
        ),
    ],
);

const MODELS: Select = (
    "model",
    "Model",
    Some(Category::Model),
    &[
        (
            "default",
            "Default (recommended)",
            Some(
                "The agent's own choice for this account, which may change as newer models arrive",
            ),
        ),
        (
            "large",
            "Large",
            Some(
                "The most capable model for complex work, slower and with the highest use of the plan's limits",
            ),
        ),
        (
            "large-1m",
            "Large with a million tokens of context",
            Some(
                "The large model with an extended context window for very long sessions; uses limits faster",
            ),
        ),
        (
            "medium",
            "Medium",
            Some("Fast and capable for everyday coding"),
        ),
        (
            "medium-1m",
            "Medium with a million tokens of context",
            Some("Medium with the extended window"),
        ),
        (
            "small",
            "Small",
            Some("The fastest model for quick answers"),
        ),
        (
            "previous-large",
            "Previous large",
            Some("The earlier large model, kept for sessions that started on it"),
        ),
        (
            "previous-medium",
            "Previous medium",
            Some("The earlier medium model"),
        ),
        (
            "previous-small",
            "Previous small",
            Some("The earlier small model"),
        ),
        (
            "research-preview-model-with-a-name-far-longer-than-any-list-is-wide",
            "research-preview-model-with-a-name-far-longer-than-any-list-is-wide",
            Some("A preview model whose name does not fit"),
        ),
        (
            "korean",
            "한국어에 맞춘 모델",
            Some("한국어 대화와 문서에 맞춘 설정으로, 설명이 한 줄에 들어가지 않을 만큼 깁니다"),
        ),
        (
            "plan-large",
            "Plan with large, build with medium",
            Some("Plans with the large model, then switches to medium to carry out the plan"),
        ),
        ("custom-1", "custom-endpoint/model-a", None),
        ("custom-2", "custom-endpoint/model-b", None),
        ("custom-3", "custom-endpoint/model-c", None),
        ("custom-4", "custom-endpoint/model-d", None),
    ],
);

const EFFORTS: Select = (
    "effort",
    "Effort",
    Some(Category::ThoughtLevel),
    &[
        ("low", "Low", None),
        ("medium", "Medium", None),
        ("high", "High", None),
        ("xhigh", "Extra high", None),
        ("max", "Max", None),
    ],
);

const ULTRACODE: Select = (
    "ultracode",
    "Ultracode",
    None,
    &[
        ("off", "Off", Some("Work in this conversation only")),
        (
            "on",
            "On",
            Some("Allow multi-agent workflows that run many agents in the background"),
        ),
    ],
);

const AGENTS: Select = (
    "agent",
    "Agent",
    None,
    &[
        (
            "default",
            "Default",
            Some("The main agent, with no subagent"),
        ),
        ("ocx-glm-5-2", "ocx-glm-5-2", Some(SUBAGENT)),
        ("ocx-gpt-5-6-luna", "ocx-gpt-5-6-luna", Some(SUBAGENT)),
        ("ocx-gpt-5-6-sol", "ocx-gpt-5-6-sol", Some(SUBAGENT)),
        ("ocx-gpt-6-astra", "ocx-gpt-6-astra", Some(SUBAGENT)),
        ("ocx-gemini-4-pro", "ocx-gemini-4-pro", Some(SUBAGENT)),
        ("ocx-qwen-4-coder", "ocx-qwen-4-coder", Some(SUBAGENT)),
        ("ocx-deepseek-v5", "ocx-deepseek-v5", Some(SUBAGENT)),
        ("ocx-kimi-k3", "ocx-kimi-k3", Some(SUBAGENT)),
        ("ocx-grok-5", "ocx-grok-5", Some(SUBAGENT)),
        ("ocx-mistral-large-3", "ocx-mistral-large-3", Some(SUBAGENT)),
        ("ocx-llama-5-405b", "ocx-llama-5-405b", Some(SUBAGENT)),
        ("Explore", "Explore", Some(SUBAGENT)),
        ("general-purpose", "general-purpose", Some(SUBAGENT)),
        (
            "korean-reviewer",
            "한국어 검토 에이전트",
            Some(
                "한국어 문서와 커밋 메시지를 검토합니다.\n둘째 줄의 설명입니다.\n\n예:\n- 맞춤법\n- 문체",
            ),
        ),
        (
            "an-agent-whose-name-is-far-longer-than-any-list-is-wide-and-must-end-early",
            "an-agent-whose-name-is-far-longer-than-any-list-is-wide-and-must-end-early",
            Some(SUBAGENT),
        ),
    ],
);

const SELECTS: [Select; 5] = [MODES, MODELS, EFFORTS, ULTRACODE, AGENTS];

/// The values in effect.
#[derive(Debug, Clone)]
pub(crate) struct Real {
    /// Each select's value, in `SELECTS`' order.
    chosen: Vec<String>,
    fast: bool,
}

impl Real {
    pub(crate) fn new() -> Self {
        Self {
            chosen: SELECTS
                .iter()
                .map(|(_, _, _, values)| values.first().map_or("", |value| value.0).to_owned())
                .collect(),
            fast: false,
        }
    }

    /// Every option, as an agent answers them.
    pub(crate) fn all(&self) -> Vec<SessionConfigOption> {
        let mut all = Vec::new();
        for (index, (id, name, category, values)) in SELECTS.iter().enumerate() {
            let current = self.chosen.get(index).cloned().unwrap_or_default();
            let options = values
                .iter()
                .map(|(value, label, description)| {
                    SessionConfigSelectOption::new(*value, *label)
                        .description(description.map(str::to_owned))
                })
                .collect::<Vec<_>>();
            let option = SessionConfigOption::select(*id, *name, current, options)
                .category(category.map(Category::schema));
            all.push(option);
            if *id == "effort" {
                // The on/off option comes after the effort, with no name.
                all.push(SessionConfigOption::boolean("fast_mode", "", self.fast));
            }
        }
        all
    }

    /// `session/set_config_option`: any listed value; answers every option.
    pub(crate) fn set(
        &mut self,
        id: &str,
        value: &SessionConfigOptionValue,
    ) -> Result<Vec<SessionConfigOption>, Error> {
        match value {
            SessionConfigOptionValue::Boolean { value } if id == "fast_mode" => {
                self.fast = *value;
                return Ok(self.all());
            }
            SessionConfigOptionValue::ValueId { value } => {
                let found = SELECTS.iter().position(|(select, _, _, values)| {
                    *select == id && values.iter().any(|(listed, _, _)| *listed == &*value.0)
                });
                if let Some(slot) = found.and_then(|index| self.chosen.get_mut(index)) {
                    *slot = value.0.to_string();
                    return Ok(self.all());
                }
            }
            // A boolean for another id, or a kind of value this fake does
            // not know (the schema may add kinds).
            SessionConfigOptionValue::Boolean { .. } | _ => {}
        }
        Err(Error::invalid_params())
    }
}
