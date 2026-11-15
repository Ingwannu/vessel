//! Sidebar filter policy over producer-owned keys. This module never parses
//! label names, derives project identity, or treats temporary absence as deletion.
//! References: sidebar-model.tsx:89–167, sidebar-project-filter.ts,
//! sidebar-labels.ts and use-sidebar-workspaces-list's host-selection gate.

/// A loaded registry can reconcile saved hosts; a cold one cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryReadiness {
    /// Keep an unmatched stored selection inert while connecting.
    Loading,
    /// Fall back to all available hosts if every selected host is gone.
    Loaded,
}
/// Resolve hosts in available-registry order. An empty preference means all.
#[must_use]
pub fn hosts<'a, K: PartialEq>(
    available: &'a [K],
    selected: &[K],
    registry: RegistryReadiness,
) -> Vec<&'a K> {
    if selected.is_empty() {
        return available.iter().collect();
    }
    let matching = available
        .iter()
        .filter(|key| selected.contains(key))
        .collect::<Vec<_>>();
    if matching.is_empty() && registry == RegistryReadiness::Loaded {
        available.iter().collect()
    } else {
        matching
    }
}
/// Intersect saved project keys without editing them. Empty means all visible
/// projects, so a saved filter resumes when its disconnected host returns.
#[must_use]
pub fn projects<'a, K: PartialEq>(saved: &'a [K], available: &[K]) -> Vec<&'a K> {
    saved.iter().filter(|key| available.contains(key)).collect()
}
/// The owner supplies normalized label identity; Unlabelled is a selection in
/// the same OR list, never a second catalog or an empty-string domain label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelChoice<K> {
    /// Match any workspace with this normalized owner key.
    Named(K),
    /// Match a workspace carrying no real normalized label.
    Unlabelled,
}
/// Multiple chosen labels use OR, including the explicit Unlabelled choice.
#[must_use]
pub fn labels<K: PartialEq>(selected: &[LabelChoice<K>], normalized: &[K]) -> bool {
    selected.is_empty()
        || selected.iter().any(|choice| match choice {
            LabelChoice::Named(key) => normalized.contains(key),
            LabelChoice::Unlabelled => normalized.is_empty(),
        })
}
/// Preserve an originally empty project for its create action. A label filter
/// has no workspace to match there; rows filtered out of a nonempty project
/// also remove its header. Project-allowlist membership is applied separately.
#[must_use]
pub fn keep_project<T, K>(original: &[T], visible: &[T], labels: &[LabelChoice<K>]) -> bool {
    !visible.is_empty() || (labels.is_empty() && original.is_empty())
}
#[cfg(test)]
mod tests;
