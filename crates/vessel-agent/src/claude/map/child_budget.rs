//! Preflight actual canonical translator insertions, before retaining child ids.
//! The parent translator keeps its existing behavior and signatures.
use super::super::wire::{Assistant, Block, RawEvent, StreamEvent};
use super::Translator;
use crate::child_ids::ChildBudget;
use std::collections::HashSet;

impl Translator {
    /// Only message-start retains a streaming id; chunks retain no extra key.
    pub(crate) fn reserve_child_stream(
        &self,
        frame: &StreamEvent,
        budget: &mut ChildBudget,
    ) -> bool {
        if let RawEvent::MessageStart { message } = &frame.event
            && !self.streamed.contains(&message.id)
        {
            return budget.retain(&message.id);
        }
        true
    }
    /// Deduplicate within the incoming frame exactly as canonical announcement does.
    pub(crate) fn reserve_child_assistant(
        &self,
        frame: &Assistant,
        budget: &mut ChildBudget,
    ) -> bool {
        let mut proposed = HashSet::new();
        for block in &frame.message.content {
            if let Block::ToolUse { id, .. } = block
                && !self.announced.contains(id)
                && proposed.insert(id.as_str())
                && !budget.retain(id)
            {
                return false;
            }
        }
        true
    }
}
