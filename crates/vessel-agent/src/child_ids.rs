//! One retained identity budget shared by native child routers, independent of drivers.
use vessel_proto::subagents::{ProviderChildId, bounds};

/// Shared across every child translator and routing map of a managed parent.
/// Counting every retained copy is deliberately conservative for duplicate ids.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ChildBudget {
    count: usize,
    bytes: usize,
}
impl ChildBudget {
    /// Maps retain both their key and value, even when the text is identical.
    pub(crate) fn pair(&mut self, key: &str, value: &str) -> bool {
        let mut proposed = *self;
        if !proposed.retain(key) || !proposed.retain(value) {
            return false;
        }
        *self = proposed;
        true
    }
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_map_value_reservation_keeps_the_entire_pair_unretained() {
        let mut budget = ChildBudget {
            count: bounds::MAX_TRACKED_IDS - 1,
            bytes: 0,
        };
        assert!(!budget.pair("one", "two"));
        assert_eq!(budget.count, bounds::MAX_TRACKED_IDS - 1);
        assert_eq!(budget.bytes, 0);
    }
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
