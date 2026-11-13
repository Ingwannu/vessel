//! The page's `data/model-rows.test.ts`.

use vessel_proto::{ModelRoute, ProfileInfo};

use crate::Lang;
use crate::models::{profile_facts, route_facts};

#[test]
fn a_route_reads_as_its_provider_model_small_model_and_api() {
    let route: ModelRoute = serde_json::from_value(serde_json::json!({
        "name": "qwen-coder",
        "provider": "openrouter",
        "api": "openai_chat_completions",
        "model": "qwen/qwen3-coder",
        "small_model": "qwen/qwen3-coder-flash",
    }))
    .unwrap();
    assert_eq!(
        route_facts(Lang::En, &route),
        [
            "openrouter",
            "qwen/qwen3-coder",
            "small: qwen/qwen3-coder-flash",
            "openai-chat-completions"
        ]
    );
}

#[test]
fn a_profile_reads_as_its_agent_where_its_models_come_from_and_what_it_sets() {
    let profile = |json| serde_json::from_value::<ProfileInfo>(json).unwrap();
    assert_eq!(
        profile_facts(
            Lang::En,
            &profile(serde_json::json!({
                "name": "claude-qwen", "harness": "claude-code", "sets": ["A", "B"], "model": "qwen-coder",
            })),
            &[]
        ),
        ["Claude Code", "models from qwen-coder", "sets 2 variables"]
    );
    assert_eq!(
        profile_facts(
            Lang::En,
            &profile(serde_json::json!({ "name": "work", "harness": "codex", "sets": [] })),
            &[]
        ),
        ["Codex", "the agent's own models"]
    );
}
