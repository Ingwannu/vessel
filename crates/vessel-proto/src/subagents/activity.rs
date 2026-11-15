//! Read-only canonical child content cannot introduce managed-session control.

use super::{ChildDescriptor, ChildGap, ProviderChildId};
use crate::AgentEvent;
use serde::{Deserialize, Serialize};

/// Canonical activity from a provider task, validated at every decode edge.
/// Boxing in the observation keeps the enclosing event finite; nested provider
/// observations and actionable permission/question events are refused here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "AgentEvent", into = "AgentEvent")]
pub struct ChildActivity(AgentEvent);

/// A provider task cannot acquire a managed-session identity or control rights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("this event is not read-only provider child activity")]
pub struct InvalidChildActivity;

impl TryFrom<AgentEvent> for ChildActivity {
    type Error = InvalidChildActivity;
    fn try_from(event: AgentEvent) -> Result<Self, Self::Error> {
        match &event {
            AgentEvent::Ready { .. }
            | AgentEvent::PermissionRequested(_)
            | AgentEvent::PermissionResolved { .. }
            | AgentEvent::QuestionAsked(_)
            | AgentEvent::QuestionResolved { .. }
            | AgentEvent::ProviderChild(_) => Err(InvalidChildActivity),
            AgentEvent::TurnStarted { .. }
            | AgentEvent::UserMessageChunk { .. }
            | AgentEvent::MessageChunk { .. }
            | AgentEvent::ThoughtChunk { .. }
            | AgentEvent::ToolCall(_)
            | AgentEvent::ToolCallUpdate(_)
            | AgentEvent::Plan { .. }
            | AgentEvent::TurnEnded { .. }
            | AgentEvent::TurnFailed { .. }
            | AgentEvent::HarnessLog { .. }
            | AgentEvent::Steered { .. } => Ok(Self(event)),
        }
    }
}
impl From<ChildActivity> for AgentEvent {
    fn from(activity: ChildActivity) -> Self {
        activity.0
    }
}
impl ChildActivity {
    /// Reuse the canonical renderer, without creating a second timeline schema.
    #[must_use]
    pub fn event(&self) -> &AgentEvent {
        &self.0
    }

    /// A supported wrapper still masks newer optional canonical inner fields.
    pub(crate) fn for_protocol(&self, protocol: crate::ProtocolVersion) -> Option<Self> {
        self.0
            .for_protocol(protocol)
            .and_then(|event| event.try_into().ok())
    }
}

/// Actual driver observations, retained in the managed parent's durable log.
/// No observation is evidence of support; a supported empty list needs the
/// explicit support observation from an implementing driver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderObservation {
    /// The active driver implements provider-child reporting.
    Supported,
    /// Complete descriptor after applying provider-specific sticky semantics.
    Descriptor {
        /// Current provider facts, with unknown fields left unknown.
        descriptor: ChildDescriptor,
    },
    /// Canonical activity for this actual provider identity.
    Activity {
        /// Provider identity scoped by the managed parent log.
        child: ProviderChildId,
        /// Read-only activity; cannot recursively wrap another observation.
        activity: Box<ChildActivity>,
    },
    /// Collection stopped retaining data at an explicit resource boundary.
    Gap {
        /// The provider/translator did not preserve the complete history.
        reason: ChildGap,
    },
}

impl ProviderObservation {
    pub(crate) fn for_protocol(&self, protocol: crate::ProtocolVersion) -> Option<Self> {
        match self {
            Self::Activity { child, activity } => {
                activity
                    .for_protocol(protocol)
                    .map(|activity| Self::Activity {
                        child: child.clone(),
                        activity: Box::new(activity),
                    })
            }
            Self::Supported | Self::Descriptor { .. } | Self::Gap { .. } => None,
        }
    }
}
