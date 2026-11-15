//! MCP policy and routing schema edge. Hidden tools cannot be called by name.

use rmcp::model::{JsonObject, Tool};
use vessel_proto::mcp::{ToolId, ToolPolicy};

use super::{ToolError, VesselTools, text};

impl VesselTools {
    /// Freeze the exact provider policy supplied by this session's launch.
    /// Running servers do not consult mutable settings between requests.
    #[must_use]
    pub fn with_policy(mut self, policy: ToolPolicy) -> Self {
        self.policy = policy;
        self
    }
    /// Accept only an explicit transient launch context from the voice producer.
    /// This is independent of core visibility and grants no playback authority.
    #[must_use]
    pub const fn with_voice_context(mut self, voice: vessel_proto::mcp::VoiceTools) -> Self {
        self.voice = voice;
        self
    }

    /// The producer uses this in addition to readiness and exact-session lease checks.
    #[must_use]
    pub const fn voice_context(&self) -> vessel_proto::mcp::VoiceTools {
        self.voice
    }
}

pub(super) fn visible(policy: &ToolPolicy) -> Vec<Tool> {
    super::tools()
        .into_iter()
        .filter(|tool| ToolId::parse(&tool.name).is_some_and(|id| policy.permits(id)))
        .collect()
}

pub(super) fn selector(args: &JsonObject) -> Result<Option<&str>, ToolError> {
    match (text(args, "machine")?, text(args, "node")?) {
        (Some(machine), Some(node)) if machine != node => Err(ToolError::Arguments(
            "`machine` and `node` name different destinations; give one".to_owned(),
        )),
        (Some(machine), _) => Ok(Some(machine)),
        (None, node) => Ok(node),
    }
}

/// Refuse a selector on a local-only tool rather than silently acting here.
pub(super) fn validate_selector(tool: ToolId, args: &JsonObject) -> Result<(), ToolError> {
    let Some(_given) = selector(args)? else {
        return Ok(());
    };
    let supported = super::tools()
        .into_iter()
        .find(|entry| entry.name == tool.as_str())
        .is_some_and(|entry| {
            entry
                .input_schema
                .get("properties")
                .and_then(|properties| properties.get("machine"))
                .is_some()
        });
    if !supported {
        return Err(ToolError::Arguments(format!(
            "`{}` is local-only; no machine selector is supported",
            tool.as_str()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use vessel_proto::mcp::{AddedId, StoredTool, ToolAccess};

    #[test]
    fn visibility_uses_the_shared_identity_table() {
        let denied = StoredTool::Known(ToolId::CreateSession);
        let policy = ToolPolicy::new(ToolAccess::Enabled, vec![denied]).unwrap();
        let names: Vec<_> = visible(&policy).into_iter().map(|tool| tool.name).collect();
        assert!(!names.iter().any(|name| name == "create_session"));
        assert!(names.iter().any(|name| name == "send_input"));
        assert_eq!(names.len(), super::super::tools().len().saturating_sub(1));
        assert!(visible(&ToolPolicy::new(ToolAccess::Disabled, Vec::new()).unwrap()).is_empty());
    }

    #[test]
    fn machine_aliases_are_explicit_and_local_only_tools_refuse_them() {
        for key in ["machine", "node"] {
            let args = json!({key:"desktop"});
            let args = args.as_object().unwrap();
            assert_eq!(selector(args).unwrap(), Some("desktop"));
            assert!(validate_selector(ToolId::CreateSession, args).is_ok());
            assert!(validate_selector(ToolId::CreateHeartbeat, args).is_err());
            assert!(validate_selector(ToolId::Added(AddedId::GetSchedule), args).is_err());
        }
        let conflict = json!({"machine":"desktop","node":"laptop"});
        assert!(selector(conflict.as_object().unwrap()).is_err());
        let wrong = json!({"machine":false});
        assert!(selector(wrong.as_object().unwrap()).is_err());
    }
}
