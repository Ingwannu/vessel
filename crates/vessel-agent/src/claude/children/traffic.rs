//! Separate canonical translators keep streamed/full repeats out of both siblings and main chat.
use super::super::wire::{Assistant, Block, StreamEvent, UserMessage};
use super::{Children, identity};
use vessel_proto::AgentEvent;
use vessel_proto::subagents::{ChildGap, ProviderChildId};

impl Children {
    fn route(&mut self, raw: &str, out: &mut Vec<AgentEvent>) -> Option<ProviderChildId> {
        let Ok(key) = raw.parse::<ProviderChildId>() else {
            out.extend(self.gap(ChildGap::IngestionBudget));
            return None;
        };
        if let Some(id) = self.aliases.get(&key) {
            return Some(id.clone());
        }
        // A task-aware CLI's declaration is the vocabulary: local_bash and
        // housekeeping must never be fabricated into subagents from their ids.
        if self.saw_tasks {
            return None;
        }
        let mut budget = self.budget;
        if !budget.pair(raw, key.as_str()) {
            out.extend(self.gap(ChildGap::IngestionBudget));
            return None;
        }
        let descriptor = identity(key.clone(), self.owners.get(&key).cloned());
        match self.descriptor(descriptor) {
            Ok(observed) => out.push(observed),
            Err(reason) => {
                out.extend(self.gap(reason));
                return None;
            }
        }
        self.budget = budget;
        self.aliases.insert(key.clone(), key.clone());
        Some(key)
    }
    pub(super) fn stream(&mut self, frame: &StreamEvent) -> Vec<AgentEvent> {
        let Some(parent) = frame.parent_tool_use_id.as_deref() else {
            return Vec::new();
        };
        let mut out = self.support();
        let Some(id) = self.route(parent, &mut out) else {
            return out;
        };
        let Some(index) = self.index(&id) else {
            return out;
        };
        let mut budget = self.budget;
        let Some(child) = self.entries.get(index) else {
            return out;
        };
        if !child.translator.reserve_child_stream(frame, &mut budget) {
            out.extend(self.gap(ChildGap::IngestionBudget));
            return out;
        }
        let mut frame = frame.clone();
        frame.parent_tool_use_id = None;
        self.budget = budget;
        if let Some(child) = self.entries.get_mut(index) {
            let events = child.translator.stream(&frame);
            out.extend(self.activity(&id, events));
        }
        out
    }
    pub(super) fn assistant(&mut self, frame: &Assistant) -> Vec<AgentEvent> {
        let Some(parent) = frame.parent_tool_use_id.as_deref() else {
            return Vec::new();
        };
        let mut out = self.support();
        let Some(id) = self.route(parent, &mut out) else {
            return out;
        };
        let Some(index) = self.index(&id) else {
            return out;
        };
        let mut budget = self.budget;
        let Some(child) = self.entries.get(index) else {
            return out;
        };
        if !child.translator.reserve_child_assistant(frame, &mut budget) {
            out.extend(self.gap(ChildGap::IngestionBudget));
            return out;
        }
        let mut owners = Vec::new();
        for block in &frame.message.content {
            if let Block::ToolUse { id: tool, .. } = block {
                let Ok(tool) = tool.parse::<ProviderChildId>() else {
                    out.extend(self.gap(ChildGap::IngestionBudget));
                    return out;
                };
                if self.owners.get(&tool).is_some_and(|owner| owner != &id) {
                    out.extend(self.gap(ChildGap::Nesting));
                    return out;
                }
                if !self.owners.contains_key(&tool) && !owners.contains(&tool) {
                    if !budget.pair(tool.as_str(), id.as_str()) {
                        out.extend(self.gap(ChildGap::IngestionBudget));
                        return out;
                    }
                    owners.push(tool);
                }
            }
        }
        for tool in owners {
            self.owners.insert(tool, id.clone());
        }
        let mut frame = frame.clone();
        frame.parent_tool_use_id = None;
        self.budget = budget;
        if let Some(child) = self.entries.get_mut(index) {
            let events = child.translator.assistant(&frame);
            out.extend(self.activity(&id, events));
        }
        out
    }
    pub(super) fn user(&mut self, frame: &UserMessage) -> Vec<AgentEvent> {
        let Some(parent) = frame.parent_tool_use_id.as_deref() else {
            return Vec::new();
        };
        let mut out = self.support();
        let Some(id) = self.route(parent, &mut out) else {
            return out;
        };
        let Some(index) = self.index(&id) else {
            return out;
        };
        let mut frame = frame.clone();
        frame.parent_tool_use_id = None;
        if let Some(child) = self.entries.get(index) {
            let events = child.translator.user(&frame);
            out.extend(self.activity(&id, events));
        }
        out
    }
}
