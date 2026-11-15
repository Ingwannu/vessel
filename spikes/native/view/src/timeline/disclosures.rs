//! Explicit disclosure choices survive changing tool status and reasoning defaults.

use std::collections::HashMap;

/// Only explicit choices are stored; new entries follow their current default.
#[derive(Debug, Default)]
pub struct Disclosures(HashMap<usize, bool>);

impl Disclosures {
    /// The person's explicit choice, or the current automatic default.
    #[must_use]
    pub fn expanded(&self, entry: usize, default: bool) -> bool {
        self.0.get(&entry).copied().unwrap_or(default)
    }

    /// Store the opposite of the state actually shown, never an inverted default.
    pub fn toggle(&mut self, entry: usize, default: bool) {
        let next = !self.expanded(entry, default);
        let _previous = self.0.insert(entry, next);
    }

    /// A different conversation has different entry identities.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::Disclosures;

    #[test]
    fn explicitly_closed_work_stays_closed_after_completion_or_preference_change() {
        let mut disclosures = Disclosures::default();
        disclosures.toggle(0, true);
        assert!(!disclosures.expanded(0, true));
        assert!(!disclosures.expanded(0, false));
        assert!(disclosures.expanded(1, true));
        disclosures.toggle(0, false);
        assert!(disclosures.expanded(0, true));
        disclosures.clear();
        assert!(!disclosures.expanded(0, false));
    }
}
