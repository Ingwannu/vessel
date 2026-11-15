//! Canonical preferences rebase atomic workspace trees, never feature resources.
use crate::shell::Shell;
use std::collections::{BTreeMap, BTreeSet};
use vessel_app_kit::ui::Context;
use vessel_app_view::layout::{Layout, LayoutKey, Modified, RememberedLayout};

impl Shell {
    /// Called with old/new canonical snapshots before replacing projected prefs.
    /// No save is emitted here: the process-wide coordinator owns publication.
    pub(crate) fn layout_preferences_applied(
        &mut self,
        old: &[RememberedLayout],
        new: &[RememberedLayout],
        cx: &mut Context<'_, Self>,
    ) {
        let mut held = BTreeSet::new();
        if let Some(form) = &self.areas.layout.rename {
            let _inserted = held.insert(form.workspace().clone());
        }
        if self.areas.layout.menu.is_open()
            && let Some((key, _)) = &self.areas.layout.menu_tab
        {
            let _inserted = held.insert(key.clone());
        }
        if (self.areas.layout.resizing.get().is_some()
            || self.areas.layout.drop_preview.get().is_some()
            || self.areas.layout.tab_drop_preview.get().is_some())
            && let Ok(key) = super::key(self)
        {
            let _inserted = held.insert(key);
        }
        if let Some(book) = self.areas.layout.book.borrow_mut().as_mut() {
            rebase(book, old, new, &held);
        }
        cx.notify();
    }
}

fn rebase(
    book: &mut BTreeMap<LayoutKey, Layout>,
    old: &[RememberedLayout],
    new: &[RememberedLayout],
    held: &BTreeSet<LayoutKey>,
) {
    let before: BTreeMap<_, _> = old.iter().map(|entry| (&entry.key, entry)).collect();
    let after: BTreeMap<_, _> = new.iter().map(|entry| (&entry.key, entry)).collect();
    let keys: BTreeSet<_> = before.keys().chain(after.keys()).copied().collect();
    for key in keys {
        let previous = before.get(key).copied();
        let incoming = after.get(key).copied();
        if previous == incoming || held.contains(key) {
            continue;
        }
        if let Some(current) = book.get(key) {
            if !current.sheets().is_empty()
                || current.tabs().any(|tab| tab.modified() == Modified::Dirty)
                || Some(RememberedLayout::new(key.clone(), current)).as_ref() != previous
            {
                continue;
            }
        } else if previous.is_some() {
            // A local removal is also a pending local change.
            continue;
        }
        if let Some(incoming) = incoming {
            let _previous = book.insert(key.clone(), incoming.layout.clone());
        } else {
            let _removed = book.remove(key);
        }
    }
}

#[cfg(test)]
mod tests;
