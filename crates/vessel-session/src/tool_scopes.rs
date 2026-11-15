//! Retain attribution without guessing from whichever turn happens to be active.
//!
//! [Decision Log]
//! - Native tool ids can repeat. Evicting a remembered id would make a late
//!   update look new, so associations are never evicted within this actor.
//! - Both count and worst-case retained bytes are bounded. Once full, new
//!   unscoped observations stay independent records; explicit provider scopes
//!   still work. This affects display only and never authorizes an operation.
use std::collections::HashMap;
use vessel_proto::timeline::ToolCallScope;
use vessel_proto::{AgentEvent, EventSeq, SessionEvent, ToolCallId};

const MAX_TOOL_ASSOCIATIONS: usize = 1024;
const MAX_TOOL_ASSOCIATION_BYTES: usize = 1_048_576;
// ToolCallId itself is legacy/unbounded. Charge its actual UTF-8 bytes plus
// the largest validated scope identity and conservative map/enum overhead.
const ASSOCIATION_OVERHEAD_BYTES: usize =
    vessel_proto::timeline::MAX_TIMELINE_IDENTITY_BYTES * 2 + 256;

#[derive(Debug, Default)]
pub(crate) struct ToolScopes {
    turn: Option<EventSeq>,
    known: HashMap<ToolCallId, Option<ToolCallScope>>,
    bytes: usize,
}

impl ToolScopes {
    pub(crate) fn observe(&mut self, event: &mut SessionEvent, seq: EventSeq) {
        let SessionEvent::Agent { event } = event else {
            return;
        };
        match event {
            AgentEvent::TurnStarted { .. } => self.turn = Some(seq),
            AgentEvent::TurnEnded { .. } | AgentEvent::TurnFailed { .. } => self.turn = None,
            AgentEvent::ToolCall(call) => {
                let scope = call.scope.get_or_insert_with(|| {
                    Box::new(self.turn.map_or(ToolCallScope::Record { seq }, |started| {
                        ToolCallScope::SessionTurn { started }
                    }))
                });
                self.remember(&call.id, scope);
            }
            AgentEvent::ToolCallUpdate(update) => {
                if update.scope.is_none() {
                    update.scope = Some(Box::new(self.lookup(&update.id, seq)));
                }
            }
            AgentEvent::PermissionRequested(request) => {
                if request.tool_scope.is_none()
                    && let Some(call) = &request.tool_call
                {
                    request.tool_scope = Some(Box::new(self.lookup(call, seq)));
                }
            }
            AgentEvent::QuestionAsked(question) => {
                if question.tool_scope.is_none()
                    && let Some(call) = &question.tool_call
                {
                    question.tool_scope = Some(Box::new(self.lookup(call, seq)));
                }
            }
            AgentEvent::Ready { .. }
            | AgentEvent::UserMessageChunk { .. }
            | AgentEvent::MessageChunk { .. }
            | AgentEvent::ThoughtChunk { .. }
            | AgentEvent::PermissionResolved { .. }
            | AgentEvent::QuestionResolved { .. }
            | AgentEvent::Plan { .. }
            | AgentEvent::Notification(_)
            | AgentEvent::Compaction(_)
            | AgentEvent::Plugin(_)
            | AgentEvent::Error { .. }
            | AgentEvent::Steered { .. }
            | AgentEvent::HarnessLog { .. } => {}
        }
    }

    fn lookup(&self, id: &ToolCallId, seq: EventSeq) -> ToolCallScope {
        self.known
            .get(id)
            .and_then(Clone::clone)
            .unwrap_or(ToolCallScope::Record { seq })
    }

    fn remember(&mut self, id: &ToolCallId, scope: &ToolCallScope) {
        if let Some(previous) = self.known.get_mut(id) {
            if previous.as_ref() != Some(scope) {
                *previous = None;
            }
            return;
        }
        let Some(charge) = id.as_str().len().checked_add(ASSOCIATION_OVERHEAD_BYTES) else {
            return;
        };
        let Some(bytes) = self.bytes.checked_add(charge) else {
            return;
        };
        if self.known.len() >= MAX_TOOL_ASSOCIATIONS || bytes > MAX_TOOL_ASSOCIATION_BYTES {
            return;
        }
        let _prior = self.known.insert(id.clone(), Some(scope.clone()));
        self.bytes = bytes;
    }
}

#[cfg(test)]
mod tests;
