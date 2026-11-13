//! A field's key for `file.find` (protocol 10): the node stops an older
//! search when a newer one arrives with the same key from the same
//! connection, and answers the older one `superseded` with no matches
//! (the page's `data/workspace/find.ts`). A field takes one key for its
//! life and sends it with every search; an answer marked `superseded` is
//! the field's to drop, since a newer answer is on its way. Nothing is
//! debounced: the node's replacement is the bound.

use vessel_proto::workspace::SearchKey;

/// The largest key the node takes.
pub const SEARCH_KEY_MAX: u32 = u32::MAX;

/// The key after `last` (the first is 1; after the largest the count starts
/// again).
#[must_use]
pub const fn next_search_key(last: u32) -> u32 {
    if last == SEARCH_KEY_MAX {
        1
    } else {
        last.saturating_add(1)
    }
}

/// Keys for the fields of one window, each different from every other
/// field's.
#[derive(Debug, Clone, Default)]
pub struct SearchKeys {
    last: u32,
}

impl SearchKeys {
    /// A key for one field, taken when the field appears.
    pub const fn next(&mut self) -> SearchKey {
        self.last = next_search_key(self.last);
        SearchKey(self.last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_from_one_and_starts_again_after_the_largest_the_node_takes() {
        assert_eq!(next_search_key(0), 1);
        assert_eq!(next_search_key(41), 42);
        assert_eq!(next_search_key(SEARCH_KEY_MAX), 1);
    }

    #[test]
    fn is_different_for_each_field_that_asks_for_one() {
        let mut keys = SearchKeys::default();
        let composer = keys.next();
        let panel = keys.next();
        assert_ne!(panel, composer);
        assert!(composer.0 > 0);
    }
}
