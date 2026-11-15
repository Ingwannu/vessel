//! Preflight actual canonical translator insertions, before retaining child ids.
//! The parent translator keeps its existing behavior and signatures.
use super::super::wire::{Assistant, Block, RawEvent, StreamEvent};
use super::Translator;
use std::collections::HashSet;
use vessel_proto::subagents::{ProviderChildId, bounds};

/// Shared across every child translator and routing map of a managed parent.
/// Counting every retained copy is deliberately conservative for duplicate ids.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ChildBudget {
    count: usize,
    bytes: usize,
}
impl ChildBudget {
    /// Reserve before inserting a string; failed reservation changes no budget.
    pub(crate) fn retain(&mut self, id: &str) -> bool {
        if id.is_empty()
            || id.len() > ProviderChildId::MAX_BYTES
            || id.chars().any(char::is_control)
        {
            return false;
        }
        let Some(count) = self
            .count
            .checked_add(1)
            .filter(|n| *n <= bounds::MAX_TRACKED_IDS)
        else {
            return false;
        };
        let Some(bytes) = self
            .bytes
            .checked_add(id.len())
            .filter(|n| *n <= bounds::MAX_TRACKED_ID_BYTES)
        else {
            return false;
        };
        self.count = count;
        self.bytes = bytes;
        true
    }
}
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_child_caches_share_count_and_actual_utf8_byte_budgets_without_eviction() {
        let mut budget = ChildBudget::default();
        let text = "x".repeat(ProviderChildId::MAX_BYTES);
        for _ in 0..bounds::MAX_TRACKED_IDS {
            assert!(budget.retain(&text));
        }
        let before = budget;
        assert!(!budget.retain("overflow"));
        assert_eq!(budget.count, before.count);
        assert_eq!(budget.bytes, before.bytes);
        assert!(!ChildBudget::default().retain("bad\nkey"));
        assert!(
            !ChildBudget::default()
                .retain(&"x".repeat(ProviderChildId::MAX_BYTES.saturating_add(1)))
        );
    }
}
