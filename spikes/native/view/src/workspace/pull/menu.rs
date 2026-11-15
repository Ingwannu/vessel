//! Where the pull request rows sit in the git menu: Paseo's secondary
//! list puts the remote actions first and its feature group after them
//! (`git/policy.ts`, `secondaryIds`: the pull request action, then the
//! direct merges), so here they follow Fetch, as a group of their own.

use vessel_proto::forge::MergeMethod;

use super::entry::PullEntry;
use crate::workspace::git::{Action, ActionRow};

/// One row of the git menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    /// A git action.
    Git(ActionRow),
    /// The pull request entry (never [`PullEntry::Hidden`]).
    Pull(PullEntry),
    /// A direct merge (`pull::merge_methods`).
    Merge(MergeMethod),
}

/// `rows` with `entry` and then `merges` after the remote actions; a
/// hidden entry adds nothing, merges included.
#[must_use]
pub fn with_pull(rows: Vec<ActionRow>, entry: PullEntry, merges: &[MergeMethod]) -> Vec<MenuEntry> {
    let at = rows
        .iter()
        .rposition(|row| matches!(row.action, Action::Quick(_)))
        .map_or(0, |last| last.saturating_add(1));
    let mut entries: Vec<MenuEntry> = rows.into_iter().map(MenuEntry::Git).collect();
    if entry != PullEntry::Hidden {
        let group = std::iter::once(MenuEntry::Pull(entry))
            .chain(merges.iter().copied().map(MenuEntry::Merge));
        let _shifted: Vec<MenuEntry> = entries.splice(at..at, group).collect();
    }
    entries
}

#[cfg(test)]
mod tests {
    //! The rows follow Fetch, merges right after the pull request entry,
    //! and the next group still starts its own.

    use vessel_proto::forge::MergeMethod;

    use super::{MenuEntry, with_pull};
    use crate::workspace::git::fixtures::status;
    use crate::workspace::git::{Action, Reading, Writing, rows};
    use crate::workspace::git_quick::GitAction;
    use crate::workspace::pull::PullEntry;

    fn listed() -> Vec<crate::workspace::git::ActionRow> {
        let mut facts = status(vec![]);
        facts.head = Some("0123456789abcdef0123456789abcdef01234567".parse().unwrap());
        rows(&Reading::Ready(facts), &Writing::default())
    }

    #[test]
    fn create_follows_fetch() {
        let entries = with_pull(listed(), PullEntry::Create, &[]);
        let at = entries
            .iter()
            .position(|entry| matches!(entry, MenuEntry::Pull(_)))
            .unwrap();
        assert!(matches!(
            entries.get(at.saturating_sub(1)),
            Some(MenuEntry::Git(row)) if row.action == Action::Quick(GitAction::Fetch)
        ));
        assert!(matches!(
            entries.get(at.saturating_add(1)),
            Some(MenuEntry::Git(row)) if row.starts_group
        ));
    }

    #[test]
    fn merges_follow_the_pull_request_entry_in_order() {
        let methods = [MergeMethod::Squash, MergeMethod::Rebase];
        let entries = with_pull(listed(), PullEntry::Create, &methods);
        let at = entries
            .iter()
            .position(|entry| matches!(entry, MenuEntry::Pull(_)))
            .unwrap();
        assert_eq!(
            entries.get(at.saturating_add(1)),
            Some(&MenuEntry::Merge(MergeMethod::Squash))
        );
        assert_eq!(
            entries.get(at.saturating_add(2)),
            Some(&MenuEntry::Merge(MergeMethod::Rebase))
        );
    }

    #[test]
    fn a_hidden_entry_adds_no_row() {
        let rows = listed();
        let count = rows.len();
        assert_eq!(
            with_pull(rows, PullEntry::Hidden, &[MergeMethod::Squash]).len(),
            count
        );
    }
}
