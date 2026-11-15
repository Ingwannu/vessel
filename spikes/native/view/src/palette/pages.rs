//! Nested choice pages project the same source-owned entries as global search.
//! No second registry is created. A page keeps its group and breadcrumb prefix;
//! disappearing contributions simply produce an empty page and Back still works.

use std::collections::BTreeSet;

use super::registration::{Contribution, GroupId, Presentation, Registry, ResultId};

/// A choice-page location, scoped to a stable contribution group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceLocation {
    group: GroupId,
    prefix: Vec<String>,
}

impl ChoiceLocation {
    /// Enter a group's first page without inventing a path label.
    #[must_use]
    pub const fn group(group: GroupId) -> Self {
        Self {
            group,
            prefix: Vec::new(),
        }
    }

    /// Visible breadcrumbs are prepared at the owning feature's language boundary.
    pub fn breadcrumbs(&self) -> impl Iterator<Item = &str> {
        self.prefix.iter().map(String::as_str)
    }

    /// One level up, or no page when leaving the group's root.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        if self.prefix.is_empty() {
            return None;
        }
        let mut parent = self.clone();
        drop(parent.prefix.pop());
        Some(parent)
    }
}

/// A row either drills into existing paths or executes the original target.
#[derive(Debug)]
pub enum ChoiceRow<'a, T> {
    /// A shared path prefix is rendered once, even if many choices use it.
    Page {
        /// The next segment's translated label.
        title: &'a str,
        /// A validated location derived only from registered paths.
        location: ChoiceLocation,
    },
    /// A leaf keeps its source identity and typed original target.
    Result {
        /// Stable selectable identity.
        id: ResultId,
        /// The original contribution.
        entry: &'a Contribution<T>,
    },
}

/// Project a nested page in registry order; global text search stays flat and
/// searchable across all breadcrumb segments, as in Paseo.
#[must_use]
pub fn choice_rows<'a, T>(
    registry: &'a Registry<T>,
    location: &ChoiceLocation,
) -> Vec<ChoiceRow<'a, T>> {
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for (id, entry) in registry.snapshot() {
        if entry.group != location.group {
            continue;
        }
        let Presentation::Choice { path, selection: _ } = &entry.presentation else {
            continue;
        };
        let segments: Vec<_> = path.segments().collect();
        if !segments
            .iter()
            .take(location.prefix.len())
            .copied()
            .eq(location.breadcrumbs())
        {
            continue;
        }
        let Some(title) = segments.get(location.prefix.len()).copied() else {
            continue;
        };
        if location.prefix.len().saturating_add(1) == segments.len() {
            rows.push(ChoiceRow::Result { id, entry });
        } else if seen.insert(title) {
            let mut next = location.clone();
            next.prefix.push(title.to_owned());
            rows.push(ChoiceRow::Page {
                title,
                location: next,
            });
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::registration::{ChoicePath, Contribution, Rank, Selection, Visibility};

    fn choice(id: &str, group: &str, path: &[&str]) -> Contribution<u8> {
        Contribution {
            id: id.parse().unwrap(),
            group: group.parse().unwrap(),
            group_rank: Rank::default(),
            section_title: None,
            rank: Rank::default(),
            keywords: Vec::new(),
            visibility: Visibility::Query,
            presentation: Presentation::Choice {
                path: ChoicePath::new(
                    path[0].to_owned(),
                    path[1..].iter().map(|part| (*part).to_owned()).collect(),
                )
                .unwrap(),
                selection: Selection::Unselected,
            },
            target: 1,
        }
    }

    #[test]
    fn shared_prefixes_drill_into_the_same_registered_leaf_targets() {
        let mut registry = Registry::default();
        let owner = registry
            .replace(
                "composer".parse().unwrap(),
                vec![
                    choice("fast", "models", &["Provider", "Fast"]),
                    choice("large", "models", &["Provider", "Large"]),
                    choice("ignored", "modes", &["Provider", "Auto"]),
                ],
            )
            .unwrap();
        let root = ChoiceLocation::group("models".parse().unwrap());
        let rows = choice_rows(&registry, &root);
        assert_eq!(rows.len(), 1);
        let ChoiceRow::Page { title, location } = &rows[0] else {
            panic!("expected a provider page")
        };
        assert_eq!(*title, "Provider");
        let location = location.clone();
        let children = choice_rows(&registry, &location);
        assert_eq!(children.len(), 2);
        let ChoiceRow::Result { id, entry } = &children[0] else {
            panic!("expected a leaf")
        };
        assert_eq!(id.entry.as_str(), "fast");
        assert_eq!(entry.target, 1);
        assert_eq!(location.parent(), Some(root.clone()));
        assert_eq!(root.parent(), None);
        registry.remove(&owner);
        assert!(choice_rows(&registry, &location).is_empty());
        assert_eq!(location.parent(), Some(root));
    }
}
