//! Approval presentation and selected-action semantics; data never grants authority.

use serde::{Deserialize, Serialize};

use super::ToolCallDetail;

/// Delivery state is owned by the session, never inferred from an action label.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PermissionDeliveryState {
    /// A new explicit response may be admitted.
    #[default]
    Pending,
    /// The owning provider is processing a correlated response.
    Answering,
    /// A response may have applied; never replay it automatically.
    OutcomeUnknown,
}
impl PermissionDeliveryState {
    /// Default omission preserves ordinary legacy request serialization.
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

/// A request's semantic kind selects its card without matching provider tool names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    /// Permission for a proposed tool operation.
    Tool,
    /// Permission to implement a proposed plan.
    Plan,
    /// A structured question.
    Question,
    /// Permission to change provider mode.
    Mode,
    /// A provider request outside the known semantic kinds.
    Other,
}

/// A provider's permission labels can differ from a tool's generic title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PermissionPresentation {
    /// Explicit semantic kind, independent of the tool category.
    pub kind: PermissionKind,
    /// Human-readable request title, when reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Explanation of the requested decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Canonical proposed work, when reported separately from the tool item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<Box<ToolCallDetail>>,
    /// Proposed plan Markdown, never inferred from an allow button.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_text: Option<String>,
}

/// The permission behavior offered by the provider, distinct from its UI styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PermissionBehavior {
    /// Accept the selected provider action.
    Allow,
    /// Refuse the selected provider action.
    Deny,
}

/// Action emphasis is presentation; it must never select a policy or default answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PermissionActionVariant {
    /// Main action.
    Primary,
    /// Alternative action.
    Secondary,
    /// Destructive or access-sensitive action.
    Danger,
}

/// Provider intent is carried to the owning driver instead of invented by clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PermissionActionIntent {
    /// Implement the proposed plan.
    Implement,
    /// Implement after the provider's own resume transition.
    ImplementResume,
    /// Dismiss the proposed action.
    Dismiss,
}

/// Semantics attached to an existing permission option, whose id stays authoritative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PermissionAction {
    /// The behavior the provider associates with this option id.
    pub behavior: PermissionBehavior,
    /// Optional display emphasis, never authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<PermissionActionVariant>,
    /// Optional provider action meaning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<PermissionActionIntent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_and_mode_requests_keep_distinct_selected_action_intent() {
        let json = r##"{"kind":"plan","title":"Implement?","description":"Review first","plan_text":"# Plan","detail":{"type":"plan","text":"# Plan"}}"##;
        let shown: PermissionPresentation = serde_json::from_str(json).unwrap();
        assert_eq!(shown.kind, PermissionKind::Plan);
        assert_eq!(shown.plan_text.as_deref(), Some("# Plan"));
        let action: PermissionAction = serde_json::from_str(
            r#"{"behavior":"allow","intent":"implement_resume","variant":"primary"}"#,
        )
        .unwrap();
        assert_eq!(action.intent, Some(PermissionActionIntent::ImplementResume));
        assert!(
            serde_json::from_str::<PermissionAction>(
                r#"{"behavior":"allow","intent":"invented_mode"}"#
            )
            .is_err()
        );
        assert!(serde_json::from_str::<PermissionPresentation>(r#"{"kind":"invented"}"#).is_err());
    }
}
