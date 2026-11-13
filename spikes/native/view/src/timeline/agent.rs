//! What each agent event does to the timeline (the page's
//! `data/timeline-agent.ts` and the agent half of `data/conversation.ts`).

use vessel_proto::{AgentEvent, EventRecord, Question};

use super::entry::{
    EntryKey, EntryKind, Permission, Prompt, TextKind, ToolView, block_text, prompt_parts,
};
use super::fold::Timeline;

impl Timeline {
    pub(super) fn agent(&mut self, record: &EventRecord, event: &AgentEvent) {
        match event {
            AgentEvent::Ready {
                harness_session,
                abilities,
            } => {
                self.facts.abilities.clone_from(abilities);
                self.push(
                    record,
                    EntryKind::Ready {
                        harness_session: harness_session.clone(),
                    },
                );
            }
            AgentEvent::TurnStarted { content } => {
                self.turns = self.turns.saturating_add(1);
                self.turn = Some(record.seq);
                let (text, attachments) = prompt_parts(content);
                self.push(
                    record,
                    EntryKind::Prompt(Prompt {
                        text,
                        replayed: false,
                        turn: self.turns,
                        attachments,
                    }),
                );
            }
            AgentEvent::UserMessageChunk { content } => {
                self.merge(record, TextKind::Prompt, &block_text(content));
            }
            AgentEvent::MessageChunk { content } => {
                self.merge(record, TextKind::Message, &block_text(content));
            }
            AgentEvent::ThoughtChunk { content } => {
                self.merge(record, TextKind::Thought, &block_text(content));
            }
            AgentEvent::ToolCall(call) => self.tool(record, ToolView::announced(call)),
            AgentEvent::ToolCallUpdate(update) => {
                let tool = ToolView::after(self.find_tool(&update.id), update);
                self.tool(record, tool);
            }
            AgentEvent::Plan { entries } => {
                self.plan = Some(entries.clone());
                self.push(
                    record,
                    EntryKind::Plan {
                        entries: entries.clone(),
                    },
                );
            }
            AgentEvent::PermissionRequested(_)
            | AgentEvent::PermissionResolved { .. }
            | AgentEvent::TurnEnded { .. }
            | AgentEvent::TurnFailed { .. }
            | AgentEvent::HarnessLog { .. }
            | AgentEvent::Steered { .. }
            | AgentEvent::QuestionAsked(_)
            | AgentEvent::QuestionResolved { .. } => self.turn_event(record, event),
        }
    }

    /// The events that ask, answer or end: permissions, questions, a
    /// turn's end, steers and the harness's log.
    fn turn_event(&mut self, record: &EventRecord, event: &AgentEvent) {
        match event {
            AgentEvent::PermissionRequested(request) => {
                let title = self
                    .find_tool(&request.tool_call)
                    .map(|tool| tool.title.clone());
                let index = self.entries.len();
                let _absent = self.permissions.insert(request.id, index);
                let _new = self.waiting.insert(index);
                self.attention = Some(record.seq);
                self.push_keyed(
                    record,
                    EntryKey::Permission(request.id),
                    EntryKind::Permission(Permission {
                        request: request.clone(),
                        title,
                        outcome: None,
                    }),
                );
            }
            AgentEvent::PermissionResolved { id, outcome } => {
                let Some(index) = self.permissions.get(id).copied() else {
                    return;
                };
                let _was = self.waiting.remove(&index);
                if let Some(entry) = self.change(index)
                    && let EntryKind::Permission(permission) = &mut entry.kind
                {
                    permission.outcome = Some(outcome.clone());
                }
            }
            AgentEvent::TurnEnded { reason, usage } => {
                self.turn = None;
                self.attention = Some(record.seq);
                self.facts.turn_usage = *usage;
                self.push(
                    record,
                    EntryKind::TurnEnded {
                        reason: *reason,
                        usage: *usage,
                    },
                );
            }
            AgentEvent::TurnFailed { message } => {
                self.turn = None;
                self.attention = Some(record.seq);
                self.push(
                    record,
                    EntryKind::TurnFailed {
                        message: message.clone(),
                    },
                );
            }
            AgentEvent::HarnessLog { line } => {
                self.push(record, EntryKind::Log { line: line.clone() });
            }
            AgentEvent::Steered { content } => {
                let (text, attachments) = prompt_parts(content);
                self.push(record, EntryKind::Steer { text, attachments });
            }
            AgentEvent::QuestionAsked(question) => self.asked(record, question),
            AgentEvent::QuestionResolved { id, outcome } => {
                let Some(index) = self.questions.get(id).copied() else {
                    return;
                };
                if let Some(entry) = self.change(index)
                    && let EntryKind::Question {
                        outcome: settled, ..
                    } = &mut entry.kind
                {
                    *settled = Some(outcome.clone());
                }
            }
            // Folded by `agent`, never handed here.
            AgentEvent::Ready { .. }
            | AgentEvent::TurnStarted { .. }
            | AgentEvent::UserMessageChunk { .. }
            | AgentEvent::MessageChunk { .. }
            | AgentEvent::ThoughtChunk { .. }
            | AgentEvent::ToolCall(_)
            | AgentEvent::ToolCallUpdate(_)
            | AgentEvent::Plan { .. } => {}
        }
    }

    fn asked(&mut self, record: &EventRecord, question: &Question) {
        let _absent = self.questions.insert(question.id, self.entries.len());
        self.attention = Some(record.seq);
        self.push_keyed(
            record,
            EntryKey::Question(question.id),
            EntryKind::Question {
                question: question.clone(),
                outcome: None,
            },
        );
    }
}
