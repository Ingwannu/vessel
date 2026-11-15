//! Stored project/workspace order, distinct from navigation inventory order.
//! References: hooks/sidebar-workspaces-view-model.ts:471–601 and
//! utils/sidebar-reorder.ts. A filtered-out identity is retained, never deleted.
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
/// Newly discovered projects append; newly discovered workspaces prepend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// Source project-order reconciliation.
    Append,
    /// Source workspace-order reconciliation.
    Prepend,
}
/// Restore only known saved positions. New items keep their structural slots
/// until reconciliation; this is not a generic sort that moves them to an end.
#[must_use]
pub fn apply<'a, K: Eq + Hash>(items: &'a [K], stored: &[K]) -> Vec<&'a K> {
    let by_key: HashMap<&K, &K> = items.iter().map(|key| (key, key)).collect();
    let mut seen = HashSet::new();
    let matched = stored
        .iter()
        .filter(|key| by_key.contains_key(key) && seen.insert(*key))
        .collect::<Vec<_>>();
    let matched_set: HashSet<&K> = matched.iter().copied().collect();
    let mut next = matched.into_iter();
    items
        .iter()
        .map(|item| {
            if matched_set.contains(item) {
                next.next()
                    .and_then(|key| by_key.get(key))
                    .copied()
                    .unwrap_or(item)
            } else {
                item
            }
        })
        .collect()
}
/// Reconcile discovery without changing unavailable saved identities. None
/// means no write is needed; source projects append and workspaces prepend.
#[must_use]
pub fn reconcile<K: Clone + Eq + Hash>(
    stored: &[K],
    visible: &[K],
    missing: Missing,
) -> Option<Vec<K>> {
    let known: HashSet<&K> = stored.iter().collect();
    let new = visible
        .iter()
        .filter(|key| !known.contains(key))
        .cloned()
        .collect::<Vec<_>>();
    if new.is_empty() {
        return None;
    }
    Some(match missing {
        Missing::Append => stored.iter().cloned().chain(new).collect(),
        Missing::Prepend => new.into_iter().chain(stored.iter().cloned()).collect(),
    })
}
/// Commit a real visible reorder, retaining hidden/stale keys afterward. This
/// differs deliberately from navigation inventory's interleaving rewrite.
#[must_use]
pub fn reordered<K: Clone + Eq + Hash>(stored: &[K], visible: &[K]) -> Option<Vec<K>> {
    let keys: HashSet<&K> = visible.iter().collect();
    if stored
        .iter()
        .filter(|key| keys.contains(key))
        .eq(visible.iter())
    {
        return None;
    }
    Some(
        visible
            .iter()
            .cloned()
            .chain(stored.iter().filter(|key| !keys.contains(key)).cloned())
            .collect(),
    )
}
/// The GPUI drag target supplies source-relative before/after intent; no pane
/// insertion identity or independent gesture physics enters this model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Insert before the hovered canonical row.
    Before,
    /// Insert after the hovered canonical row.
    After,
}
/// Move one unique canonical key inside the current visible group. A stale or
/// same-key drop is a no-op, so a reconnect cannot invent another workspace.
#[must_use]
pub fn moved<K: Clone + PartialEq>(
    visible: &[K],
    key: &K,
    anchor: &K,
    side: Side,
) -> Option<Vec<K>> {
    if key == anchor || !visible.contains(key) || !visible.contains(anchor) {
        return None;
    }
    let mut next = visible
        .iter()
        .filter(|known| *known != key)
        .cloned()
        .collect::<Vec<_>>();
    let at = next.iter().position(|known| known == anchor)?;
    let at = match side {
        Side::Before => at,
        Side::After => at.checked_add(1)?,
    };
    // The anchor was found in this unchanged vector; after is at most len.
    next.insert(at, key.clone());
    (next != visible).then_some(next)
}
#[cfg(test)]
mod tests;
