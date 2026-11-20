//! The projects' order in the sidebar's project tree, as dragged (TRACKER
//! 811), as Paseo's (`stores/sidebar-order-store.ts`, `projectOrder`).
//!
//! [Decision Log]
//! - Purpose: a person puts their projects in their own order by dragging
//!   a header, and the order stays across launches, as Paseo keeps it per
//!   device.
//! - Before: the tree listed projects by name only (TRACKER 682's stated
//!   difference).
//! - Choice (the coordinator's ruling, Q2): the native document keeps the
//!   order only once a drop changed it, so the document the page and the
//!   bridge share is unchanged until then (as `folded_projects`). A drop
//!   writes the order it left, followed by the placed projects it did not
//!   show (Paseo's `mergeWithRemainder`: a machine out of reach keeps its
//!   projects' places). A project never placed follows the placed ones, by
//!   name.
//! - Alternative left: Paseo appends every project it sees to its stored
//!   order at once (`appendMissingOrderKeys`), so before any drag it lists
//!   projects in the order they first appeared, and the document changes
//!   without a drag. Stated difference: before a drag Vessel lists them by
//!   name; after one, a new project goes to the end in both.
//! - Kept tidy as folds are ([`ProjectKeyed`]); Paseo never prunes its keys
//!   (stated, not visible).

use serde::{Deserialize, Serialize};
use vessel_proto::ProjectId;

use super::projects::{ProjectKey, ProjectKeyed};

/// The placed projects, first to last.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectOrder(Vec<ProjectKey>);

impl ProjectOrder {
    /// `keys` in their order (a drag's order while it is under way).
    #[must_use]
    pub fn of(keys: Vec<ProjectKey>) -> Self {
        Self(keys)
    }

    /// The placed projects, first to last.
    #[must_use]
    pub fn keys(&self) -> &[ProjectKey] {
        &self.0
    }

    /// Whether none is placed (the document leaves the field out).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Where `project` on `machine` stands, first is 0, if it is placed.
    #[must_use]
    pub fn place(&self, machine: &str, project: ProjectId) -> Option<usize> {
        self.0.iter().position(|key| key.is(machine, project))
    }

    /// A drop: `before` is the order shown when the drag started, `after`
    /// the order it left. When they differ, `after` becomes the order,
    /// followed by the placed projects it does not hold; whether the order
    /// changed.
    pub fn dropped(&mut self, before: &[ProjectKey], after: &[ProjectKey]) -> bool {
        if before == after {
            return false;
        }
        let mut order: Vec<ProjectKey> = Vec::with_capacity(self.0.len().max(after.len()));
        for key in after.iter().chain(&self.0) {
            if !order.contains(key) {
                order.push(key.clone());
            }
        }
        let changed = order != self.0;
        self.0 = order;
        changed
    }
}

impl ProjectKeyed for ProjectOrder {
    fn keep_listed(&mut self, machine: &str, listed: &[ProjectId]) -> bool {
        let before = self.0.len();
        self.0
            .retain(|key| key.machine != machine || listed.contains(&key.project));
        self.0.len() != before
    }

    fn rename_machine(&mut self, from: &str, to: &str) -> bool {
        let mut any = false;
        for key in &mut self.0 {
            if key.machine == from {
                to.clone_into(&mut key.machine);
                any = true;
            }
        }
        any
    }
}

#[cfg(test)]
mod tests {
    //! The order a drop leaves, how it is kept tidy, and where a project
    //! stands.

    use vessel_proto::ProjectId;

    use super::ProjectOrder;
    use crate::prefs::{ProjectKey, ProjectKeyed};

    fn keys() -> (ProjectKey, ProjectKey, ProjectKey) {
        (
            ProjectKey::new("laptop", ProjectId::random()),
            ProjectKey::new("laptop", ProjectId::random()),
            ProjectKey::new("gpu-box", ProjectId::random()),
        )
    }

    #[test]
    fn a_drop_that_moves_nothing_writes_nothing() {
        let (shop, docs, _) = keys();
        let mut order = ProjectOrder::default();
        let shown = [shop.clone(), docs.clone()];
        assert!(!order.dropped(&shown, &shown));
        assert!(order.is_empty());
        assert_eq!(order.place("laptop", shop.project), None);
    }

    #[test]
    fn a_drop_places_what_it_shows_and_keeps_the_places_it_does_not() {
        let (shop, docs, model) = keys();
        let mut order = ProjectOrder::default();
        assert!(order.dropped(
            &[shop.clone(), docs.clone(), model.clone()],
            &[docs.clone(), model.clone(), shop.clone()],
        ));
        assert_eq!(order.place("laptop", docs.project), Some(0));
        assert_eq!(order.place("gpu-box", model.project), Some(1));
        assert_eq!(order.place("laptop", shop.project), Some(2));
        // gpu-box out of reach: its project is not shown, and keeps its
        // place after the ones the drop shows (Paseo's merge).
        assert!(order.dropped(&[docs.clone(), shop.clone()], &[shop.clone(), docs.clone()]));
        assert_eq!(order, ProjectOrder::of(vec![shop, docs, model]));
    }

    #[test]
    fn the_order_follows_the_lists_and_the_book() {
        let (shop, docs, model) = keys();
        let mut order = ProjectOrder::of(vec![shop.clone(), docs.clone(), model.clone()]);
        // A whole list without shop drops its place, nothing else.
        assert!(order.keep_listed("laptop", &[docs.project]));
        assert!(!order.keep_listed("laptop", &[docs.project]));
        assert_eq!(order.place("laptop", docs.project), Some(0));
        // A rename keeps the places under the new name.
        assert!(order.rename_machine("laptop", "workstation"));
        assert_eq!(order.place("workstation", docs.project), Some(0));
        assert!(!order.rename_machine("laptop", "elsewhere"));
        // A forgotten machine's places go.
        assert!(order.forget_machine("gpu-box"));
        assert_eq!(order.place("gpu-box", model.project), None);
        assert!(!order.forget_machine("gpu-box"));
    }

    #[test]
    fn the_document_holds_the_keys_as_folds_do() {
        let (shop, _, _) = keys();
        let order = ProjectOrder::of(vec![shop.clone()]);
        let text = serde_json::to_string(&order).unwrap();
        assert_eq!(
            text,
            format!(r#"[{{"machine":"laptop","project":"{}"}}]"#, shop.project)
        );
        assert_eq!(serde_json::from_str::<ProjectOrder>(&text).unwrap(), order);
    }
}
