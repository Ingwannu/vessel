//! What each agent event does to the timeline (the page's
//! `data/timeline-agent.ts` and the agent half of `data/conversation.ts`).

use vessel_proto::{AgentEvent, EventRecord, PlanEntry, Question};

use super::entry::{
    EntryKey, EntryKind, Permission, Prompt, TextKind, ToolView, block_text, prompt_parts,
};
use super::fold::Timeline;
use super::plan::{PlanActivity, all_pending, plan_activities};

impl Timeline {
    pub(super) fn agent(&mut self, record: &EventRecord, event: &AgentEvent) {
        match event {
            AgentEvent::Ready {
                harness_session,
                abilities,
                ..
            } => {
                self.facts.abilities.clone_from(abilities);
                self.push(
                    record,
                    EntryKind::Ready {
                        harness_session: harness_session.clone(),
                    },
                );
            }
            AgentEvent::TurnStarted { content, .. } => {
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
            AgentEvent::UserMessageChunk { content, .. } => {
                self.merge(record, TextKind::Prompt, &block_text(content));
            }
            AgentEvent::MessageChunk { content, .. } => {
                self.merge(record, TextKind::Message, &block_text(content));
            }
            AgentEvent::ThoughtChunk { content, .. } => {
                self.merge(record, TextKind::Thought, &block_text(content));
            }
            AgentEvent::ToolCall(call) => self.tool(record, ToolView::announced(call)),
            AgentEvent::ToolCallUpdate(update) => {
                let tool = ToolView::after(self.find_tool(&update.id), update);
                self.tool(record, tool);
            }
            AgentEvent::Plan { entries, .. } => self.planned(record, entries),
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
            AgentEvent::PermissionResolved { id, outcome, .. } => {
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
            // `point` (protocol 14): where a rewind or a fork can return
            // to; the turn's end is the anchor (`timeline::rewind`).
            AgentEvent::TurnEnded {
                reason,
                usage,
                point,
                ..
            } => {
                self.turn = None;
                self.attention = Some(record.seq);
                self.facts.turn_usage = *usage;
                self.push(
                    record,
                    EntryKind::TurnEnded {
                        reason: *reason,
                        usage: *usage,
                        point: point.is_some(),
                    },
                );
            }
            AgentEvent::TurnFailed { message, .. } => {
                self.turn = None;
                self.attention = Some(record.seq);
                self.push(
                    record,
                    EntryKind::TurnFailed {
                        message: message.clone(),
                    },
                );
            }
            AgentEvent::HarnessLog { line, .. } => {
                self.push(record, EntryKind::Log { line: line.clone() });
            }
            AgentEvent::Steered { content, .. } => {
                let (text, attachments) = prompt_parts(content);
                self.push(record, EntryKind::Steer { text, attachments });
            }
            AgentEvent::QuestionAsked(question) => self.asked(record, question),
            AgentEvent::QuestionResolved { id, outcome, .. } => {
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

    /// A plan update: a row for each thing that changed in it (Paseo's
    /// task activities, `plan.rs`).
    fn planned(&mut self, record: &EventRecord, entries: &[PlanEntry]) {
        let previous = self.plan.replace(entries.to_vec()).unwrap_or_default();
        let activities = plan_activities(&previous, entries);
        if activities.is_empty() {
            // Nothing a person reads changed: the last row shows the plan
            // as it now stands.
            if let Some(index) = self.plan_row
                && let Some(entry) = self.change(index)
                && let EntryKind::Plan { entries: shown, .. } = &mut entry.kind
            {
                entries.clone_into(shown);
            }
            return;
        }
        // A step added to a list just created, all still to do, is part of
        // the creation.
        let last = self.entries.len().checked_sub(1);
        if matches!(activities.as_slice(), [PlanActivity::Added(_)])
            && all_pending(entries)
            && last.is_some()
            && self.plan_row == last
            && let Some(index) = last
            && let Some(entry) = self.change(index)
            && let EntryKind::Plan {
                entries: shown,
                activity: activity @ PlanActivity::Created { .. },
            } = &mut entry.kind
        {
            entries.clone_into(shown);
            *activity = PlanActivity::Created {
                count: entries.len(),
            };
            return;
        }
        for (step, activity) in (0_u32..).zip(activities) {
            self.plan_row = Some(self.entries.len());
            self.push_keyed(
                record,
                EntryKey::Plan {
                    seq: record.seq,
                    step,
                },
                EntryKind::Plan {
                    entries: entries.to_vec(),
                    activity,
                },
            );
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
