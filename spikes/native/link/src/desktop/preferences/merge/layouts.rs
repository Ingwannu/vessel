//! A validated workspace tree is atomic; unrelated workspace records are independent.

use vessel_app_view::layout::RememberedLayout;

pub(super) fn apply(
    current: &mut Vec<RememberedLayout>,
    before: &[RememberedLayout],
    after: &[RememberedLayout],
) {
    current.retain(|entry| {
        !before.iter().any(|old| old.key == entry.key)
            || after.iter().any(|new| new.key == entry.key)
    });
    for new in after {
        if before.iter().find(|old| old.key == new.key) == Some(new) {
            continue;
        }
        match current.iter_mut().find(|entry| entry.key == new.key) {
            Some(current) => current.clone_from(new),
            None => current.push(new.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Prefs;
    use vessel_app_view::layout::{ExplorerVisibility, HostAlias, Layout, LayoutKey};
    use vessel_proto::SessionId;

    fn record() -> RememberedLayout {
        RememberedLayout::new(
            LayoutKey::Session {
                machine: HostAlias::parse("fixture").unwrap(),
                session: SessionId::random(),
            },
            &Layout::default(),
        )
    }

    #[test]
    fn stale_unrelated_workspace_edits_and_removals_preserve_the_other_workspace() {
        let first = record();
        let second = record();
        let before = Prefs {
            layouts: vec![first.clone(), second.clone()],
            ..Prefs::default()
        };
        let mut edit_first = before.clone();
        edit_first.layouts[0]
            .layout
            .show_explorer(ExplorerVisibility::Shown);
        let mut edit_second = before.clone();
        edit_second.layouts[1]
            .layout
            .show_explorer(ExplorerVisibility::Shown);
        let current = super::super::apply(&before, &before, &edit_first).unwrap();
        let next = super::super::apply(&current, &before, &edit_second).unwrap();
        assert_eq!(next.layouts[0], edit_first.layouts[0]);
        assert_eq!(next.layouts[1], edit_second.layouts[1]);
        let mut remove_first = before.clone();
        let _removed = remove_first.layouts.remove(0);
        let removed = super::super::apply(&next, &before, &remove_first).unwrap();
        assert_eq!(removed.layouts, vec![edit_second.layouts[1].clone()]);
    }

    #[test]
    fn concurrently_created_distinct_workspaces_are_not_whole_list_replacements() {
        let before = Prefs::default();
        let first = record();
        let second = record();
        let after_first = Prefs {
            layouts: vec![first.clone()],
            ..before.clone()
        };
        let after_second = Prefs {
            layouts: vec![second.clone()],
            ..before.clone()
        };
        let current = super::super::apply(&before, &before, &after_first).unwrap();
        let next = super::super::apply(&current, &before, &after_second).unwrap();
        assert_eq!(next.layouts, vec![first, second]);
    }
}
