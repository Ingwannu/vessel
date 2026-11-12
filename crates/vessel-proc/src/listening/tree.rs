//! [`Trees`]: which processes descend from which root.

use std::collections::{HashMap, VecDeque};

/// The processes of the trees rooted at a scan's roots, each with its
/// root, built from (process, parent) pairs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trees {
    root_of: HashMap<u32, u32>,
    truncated: bool,
}

impl Trees {
    /// Walk down from each root through `pairs` (process, parent), at most
    /// `limit` processes in all. A root is in its own tree even when the
    /// pairs do not name it (it may have just exited). A process two roots
    /// reach belongs to the first.
    #[must_use]
    pub fn build(roots: &[u32], pairs: impl IntoIterator<Item = (u32, u32)>, limit: usize) -> Self {
        let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
        for (pid, parent) in pairs {
            // A process is never its own child; pid 0 and 1 adopt orphans
            // and are never a root of ours.
            if pid != parent {
                children.entry(parent).or_default().push(pid);
            }
        }
        let mut root_of = HashMap::new();
        let mut truncated = false;
        for &root in roots {
            let mut queue = VecDeque::from([root]);
            while let Some(pid) = queue.pop_front() {
                if root_of.contains_key(&pid) {
                    continue;
                }
                if root_of.len() >= limit {
                    truncated = true;
                    break;
                }
                let _new = root_of.insert(pid, root);
                if let Some(below) = children.get(&pid) {
                    queue.extend(below.iter().copied());
                }
            }
        }
        Self { root_of, truncated }
    }

    /// The root of `pid`'s tree, if it is in one.
    #[must_use]
    pub fn root_of(&self, pid: u32) -> Option<u32> {
        self.root_of.get(&pid).copied()
    }

    /// Every process in the trees.
    pub fn processes(&self) -> impl Iterator<Item = u32> + '_ {
        self.root_of.keys().copied()
    }

    /// How many processes the trees hold.
    #[must_use]
    pub fn len(&self) -> usize {
        self.root_of.len()
    }

    /// Whether the trees hold nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.root_of.is_empty()
    }

    /// Whether the walk stopped at its limit.
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descendants_belong_to_their_root() {
        // 10 -> 11 -> 12, 10 -> 13; 20 -> 21; 30 elsewhere.
        let pairs = [(11, 10), (12, 11), (13, 10), (21, 20), (31, 30), (10, 1)];
        let trees = Trees::build(&[10, 20], pairs, 100);
        assert_eq!(trees.root_of(12), Some(10));
        assert_eq!(trees.root_of(13), Some(10));
        assert_eq!(trees.root_of(21), Some(20));
        assert_eq!(trees.root_of(31), None);
        assert_eq!(trees.root_of(1), None, "the parent of a root is not in it");
        assert_eq!(trees.len(), 6);
        assert!(!trees.truncated());
    }

    #[test]
    fn the_walk_stops_at_its_limit() {
        let pairs = (2..1000).map(|pid| (pid, pid - 1));
        let trees = Trees::build(&[1], pairs, 10);
        assert_eq!(trees.len(), 10);
        assert!(trees.truncated());
    }

    #[test]
    fn a_root_the_pairs_do_not_name_is_its_own_tree() {
        let trees = Trees::build(&[42], [], 10);
        assert_eq!(trees.root_of(42), Some(42));
    }
}
