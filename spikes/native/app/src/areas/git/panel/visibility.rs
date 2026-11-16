//! Current prepaint observations veto eviction; they never admit an operation.
//! Reuse actual section masks and the sole pane visibility query, not row ranges.
use std::{cell::RefCell, collections::HashSet, rc::Rc};
use vessel_proto::workspace::RelPath;

/// Only retained resources are marked, so this set shares the global slot bound.
#[derive(Clone, Default)]
pub(super) struct VisibleBodies(Rc<RefCell<HashSet<RelPath>>>);
impl VisibleBodies {
    pub(super) fn clear(&self) {
        self.0.borrow_mut().clear();
    }
    pub(super) fn mark(&self, path: &RelPath) {
        let _new = self.0.borrow_mut().insert(path.clone());
    }
    pub(super) fn contains(&self, path: &RelPath) -> bool {
        self.0.borrow().contains(path)
    }
    pub(super) fn remove(&self, path: &RelPath) {
        let _old = self.0.borrow_mut().remove(path);
    }
}
