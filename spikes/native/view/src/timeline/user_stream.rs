//! Human chunk continuity matches the canonical prompt-index classifier.
//! Metadata cannot create duplicate prompts; real content and lifecycle boundaries
//! end the stream even when they add no visible row (notably terminal events).

use vessel_proto::{AgentEvent, SessionEvent};

pub(super) const fn preserves(event: &SessionEvent) -> bool {
    match event {
        SessionEvent::Agent { event } => match event {
            AgentEvent::UserMessageChunk { .. } | AgentEvent::HarnessLog { .. } => true,
            AgentEvent::Ready { .. }
            | AgentEvent::TurnStarted { .. }
            | AgentEvent::Steered { .. }
            | AgentEvent::MessageChunk { .. }
            | AgentEvent::ThoughtChunk { .. }
            | AgentEvent::ToolCall(_)
            | AgentEvent::ToolCallUpdate(_)
            | AgentEvent::Plan { .. }
            | AgentEvent::PermissionRequested(_)
            | AgentEvent::PermissionResolved { .. }
            | AgentEvent::TurnEnded { .. }
            | AgentEvent::TurnFailed { .. }
            | AgentEvent::QuestionAsked(_)
            | AgentEvent::QuestionResolved { .. } => false,
        },
        SessionEvent::Rewound { .. } | SessionEvent::Status { .. } | SessionEvent::Terminal(_) => {
            false
        }
        SessionEvent::ProfileChanged { .. }
        | SessionEvent::ModelCall(_)
        | SessionEvent::OptionsChanged { .. }
        | SessionEvent::Renamed { .. }
        | SessionEvent::QueueChanged { .. }
        | SessionEvent::CommandsChanged { .. }
        | SessionEvent::UsageChanged { .. }
        | SessionEvent::Parked { .. }
        | SessionEvent::Woken
        | SessionEvent::Forked { .. } => true,
    }
}

#[cfg(test)]
mod tests;
