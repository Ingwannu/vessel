//! Settings › Shortcuts' rows (the page's `data/shortcuts/listing.ts`):
//! each action with its keys in the platform's words, whether the person
//! changed it, whether it is fixed, and which other actions hold one of
//! its keys; with the search over names and keys and the "Modified"
//! filter.

use std::collections::BTreeMap;

use super::keys::{Binding, Platform, binding_words, overlaps};
use super::registry::{Action, ShortcutGroup, bindings_on, jump_shown, modified};
use crate::prefs::{ActionId, Bindings};
use crate::text::ShortcutWords;

/// One action's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutRow {
    /// The action.
    pub action: Action,
    /// Its name.
    pub label: &'static str,
    /// Its keys (none for a fixed action).
    pub bindings: Vec<Binding>,
    /// Each binding as the platform writes it ("⌘K", "Ctrl+Shift+B").
    pub words: Vec<String>,
    /// Whether the person changed its keys.
    pub modified: bool,
    /// Whether its keys are fixed.
    pub fixed: bool,
    /// Other actions holding one of these keys (an older override, or a
    /// newer default): shown on the row, never resolved silently.
    pub clashes: Vec<Action>,
}

/// Which rows are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShortcutFilter {
    /// Every action.
    #[default]
    All,
    /// Only those the person changed.
    Modified,
}

fn clashes_of(
    action: Action,
    bindings: &[Binding],
    overrides: &BTreeMap<ActionId, Bindings>,
    platform: Platform,
) -> Vec<Action> {
    Action::ALL
        .into_iter()
        .filter(|other| {
            *other != action
                && !other.fixed()
                && bindings_on(*other, overrides, platform)
                    .iter()
                    .any(|theirs| bindings.iter().any(|ours| overlaps(ours, theirs, platform)))
        })
        .collect()
}

/// `action`'s row.
#[must_use]
pub fn shortcut_row(
    action: Action,
    overrides: &BTreeMap<ActionId, Bindings>,
    platform: Platform,
    words: &ShortcutWords,
) -> ShortcutRow {
    let fixed = action.fixed();
    let bindings = if fixed {
        Vec::new()
    } else {
        bindings_on(action, overrides, platform)
    };
    let shown = if fixed {
        let jump = Binding {
            first: jump_shown(),
            then: None,
        };
        vec![binding_words(&jump, platform, &words.then)]
    } else {
        bindings
            .iter()
            .map(|binding| binding_words(binding, platform, &words.then))
            .collect()
    };
    let clashes = if fixed {
        Vec::new()
    } else {
        clashes_of(action, &bindings, overrides, platform)
    };
    ShortcutRow {
        action,
        label: words.label(action),
        bindings,
        words: shown,
        modified: modified(action, overrides),
        fixed,
        clashes,
    }
}

/// Whether `row` answers a search for `query` (by name or keys).
fn found(row: &ShortcutRow, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    needle.is_empty()
        || row.label.to_lowercase().contains(&needle)
        || row
            .words
            .iter()
            .any(|words| words.to_lowercase().contains(&needle))
}

/// The rows of `group` the list shows.
#[must_use]
pub fn shortcut_rows(
    group: ShortcutGroup,
    overrides: &BTreeMap<ActionId, Bindings>,
    platform: Platform,
    query: &str,
    filter: ShortcutFilter,
    words: &ShortcutWords,
) -> Vec<ShortcutRow> {
    Action::in_group(group)
        .map(|action| shortcut_row(action, overrides, platform, words))
        .filter(|row| filter == ShortcutFilter::All || row.modified)
        .filter(|row| found(row, query))
        .collect()
}

/// How many actions differ from their defaults.
#[must_use]
pub fn modified_count(overrides: &BTreeMap<ActionId, Bindings>) -> usize {
    Action::ALL
        .into_iter()
        .filter(|action| modified(*action, overrides))
        .count()
}
