//! Settings > Models (the page's `data/model-rows.ts`), read only: a
//! machine's model routes and its profiles, each as a few facts.

pub mod page;
mod users;
pub use users::{User, users};

use vessel_proto::agents::AgentInfo;
use vessel_proto::{ModelApi, ModelRoute, ModelRouteName, ProfileInfo};

use crate::Lang;
use crate::agent_status::agent_label;

/// A model API's name, as `vessel` writes it.
#[must_use]
pub const fn api_name(api: ModelApi) -> &'static str {
    match api {
        ModelApi::AnthropicMessages => "anthropic-messages",
        ModelApi::OpenAiResponses => "openai-responses",
        ModelApi::OpenAiChatCompletions => "openai-chat-completions",
    }
}

/// A route's facts: its provider, its model, its small model, its API.
#[must_use]
pub fn route_facts(lang: Lang, route: &ModelRoute) -> Vec<String> {
    [
        Some(route.provider.to_string()),
        Some(route.model.as_str().to_owned()),
        route
            .small_model
            .as_ref()
            .map(|small| lang.machines().small_model(small.as_str())),
        Some(api_name(route.api).to_owned()),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// A profile's facts: its agent, where its models come from, how many
/// variables it sets.
#[must_use]
pub fn profile_facts(lang: Lang, profile: &ProfileInfo, agents: &[AgentInfo]) -> Vec<String> {
    let words = lang.machines();
    let sets = profile.sets.len();
    [
        Some(agent_label(&profile.harness, agents)),
        Some(words.models_from(profile.model.as_ref().map(ModelRouteName::as_str))),
        (sets > 0).then(|| words.sets(sets)),
    ]
    .into_iter()
    .flatten()
    .collect()
}
