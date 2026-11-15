//! Model use is derived only from profiles the node lists. An absent use
//! is not a capability verdict about the agent or provider.
use crate::agent_status::agent_label;
use vessel_proto::agents::AgentInfo;
use vessel_proto::{HarnessRef, ModelRoute, ProfileInfo, ProfileName};

/// One profile explicitly configured to use a model route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// Stable profile identity, separate from its optional display label.
    pub profile: ProfileName,
    /// Display label as the node reports it, or the profile identifier.
    pub label: String,
    /// Stable agent identity, including added agents.
    pub agent: HarnessRef,
    /// User-facing agent label from the same node's definitions.
    pub agent_label: String,
}
/// The node's configured users, in its profile-list order. A different
/// profile with the same agent remains a separate account/configuration.
#[must_use]
pub fn users(route: &ModelRoute, profiles: &[ProfileInfo], agents: &[AgentInfo]) -> Vec<User> {
    profiles
        .iter()
        .filter(|profile| profile.model.as_ref() == Some(&route.name))
        .map(|profile| User {
            profile: profile.name.clone(),
            label: profile
                .label
                .as_ref()
                .map_or_else(|| profile.name.to_string(), ToString::to_string),
            agent: profile.harness.clone(),
            agent_label: agent_label(&profile.harness, agents),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn only_explicit_matching_route_references_are_users_and_accounts_stay_distinct() {
        // Same route/profile shape as web/src/data/model-rows.test.ts.
        let route=serde_json::from_value(json!({"name":"qwen-coder","provider":"openrouter","api":"openai_chat_completions","model":"qwen/qwen3-coder"})).unwrap();
        let profiles: Vec<ProfileInfo> = serde_json::from_value(json!([
            {"name":"claude-qwen","label":"Work","harness":"claude-code","sets":["A","B"],"model":"qwen-coder"},
            {"name":"personal","harness":"claude-code","sets":[],"model":"qwen-coder"},
            {"name":"qwen-coder","harness":"codex","sets":[]},
            {"name":"different","harness":"codex","sets":[],"model":"other"}
        ])).unwrap();
        let used = users(&route, &profiles, &[]);
        assert_eq!(
            used.iter()
                .map(|user| user.profile.as_str())
                .collect::<Vec<_>>(),
            ["claude-qwen", "personal"]
        );
        assert_eq!(
            used.iter()
                .map(|user| user.label.as_str())
                .collect::<Vec<_>>(),
            ["Work", "personal"]
        );
        assert_eq!(
            used.iter()
                .map(|user| user.agent_label.as_str())
                .collect::<Vec<_>>(),
            ["Claude Code", "Claude Code"]
        );
    }
    #[test]
    fn a_nonrouted_added_profile_keeps_its_label_without_implying_route_use() {
        let route=serde_json::from_value(json!({"name":"remote","provider":"example","api":"openai_responses","model":"example-model"})).unwrap();
        let profiles: Vec<ProfileInfo> = serde_json::from_value(json!([
            {"name":"account","harness":{"added":"example-agent"},"sets":[]}
        ]))
        .unwrap();
        let agents: Vec<AgentInfo> = serde_json::from_value(json!([
            {"agent":{"added":"example-agent"},"detail":{"kind":"added","added":{"label":"한글 Agent","program":"run","source":{"from":"command"}}}}
        ])).unwrap();
        assert_eq!(
            crate::models::profile_facts(crate::Lang::En, &profiles[0], &agents)[0],
            "한글 Agent"
        );
        assert!(users(&route, &profiles, &agents).is_empty());
    }
}
