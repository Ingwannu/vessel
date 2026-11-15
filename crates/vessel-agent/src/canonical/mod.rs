//! Typed provider input decoding shared by canonical tools and child routing.
//! JSON trees stay at this decoder edge; consumers receive only dedicated bodies.

mod inputs;
mod snapshots;

pub(crate) use inputs::{for_kind, named, unknown};
pub(crate) use snapshots::Snapshots;

use serde::Deserialize;
use serde_json::Value;
use vessel_proto::timeline::{SubAgentDetail, TimelineName};

#[derive(Deserialize)]
struct TaskInput {
    #[serde(default)]
    description: Option<String>,
    #[serde(default, alias = "subAgentType")]
    subagent_type: Option<TimelineName>,
}

/// Actual Task/Agent tool inputs share this one typed child-launch decoder.
/// Callers select it only for native known tool names, never from user display text.
pub(crate) fn sub_agent_input(input: &Value) -> Option<SubAgentDetail> {
    let input: TaskInput = serde_json::from_value(input.clone()).ok()?;
    Some(SubAgentDetail {
        sub_agent_type: input.subagent_type,
        description: input.description,
        child_session_id: None,
        log: None,
        actions: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn child_metadata_is_typed_without_claiming_a_node_session_identity() {
        let detail = sub_agent_input(&json!({"description":"Inspect module","subagent_type":"Explore","prompt":"provider task body"})).unwrap();
        assert_eq!(detail.description.as_deref(), Some("Inspect module"));
        assert_eq!(detail.sub_agent_type.unwrap().as_str(), "Explore");
        assert!(detail.child_session_id.is_none());
        assert!(sub_agent_input(&json!({"subagent_type": "bad\nname"})).is_none());
    }
}
