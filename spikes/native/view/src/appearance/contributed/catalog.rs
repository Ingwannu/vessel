//! Pure presentation over the plugin registry owner's eligible snapshots.
use super::{ContributedChoice, PluginThemeId, PreferredHosts};
use crate::prefs::{Appearance, Theme};
use std::collections::BTreeMap;
use vessel_proto::plugins::{PluginAddress, ThemeContribution};

/// Borrowed eligible contribution. Runtime owner supplies current snapshots;
/// this presentation layer never discovers hosts or validates generations.
#[derive(Debug, Clone, Copy)]
pub struct Candidate<'a> {
    /// Exact eligible installation used by this displayed contribution.
    pub address: &'a PluginAddress,
    /// Already validated semantic palette and display name.
    pub theme: &'a ThemeContribution,
}
impl Candidate<'_> {
    /// Logical dedup key excludes host and runtime generation.
    #[must_use]
    pub fn key(self) -> PluginThemeId {
        PluginThemeId::new(self.address.plugin.clone(), self.theme.id.clone())
    }
}
/// One deduplicated choice, preserving its first logical appearance order.
#[derive(Debug)]
pub struct Entry<'a> {
    key: PluginThemeId,
    candidate: Candidate<'a>,
}
impl<'a> Entry<'a> {
    /// Persist this stable identity rather than the runtime address.
    #[must_use]
    pub const fn key(&self) -> &PluginThemeId {
        &self.key
    }
    /// The preferred eligible host, or the first eligible host.
    #[must_use]
    pub const fn candidate(&self) -> Candidate<'a> {
        self.candidate
    }
}
/// Effective palette, without mutating an unavailable saved selection.
#[derive(Debug, Clone, Copy)]
pub enum Resolution<'a> {
    /// A fixed built-in or System following ambient appearance.
    Builtin(Theme),
    /// A current eligible theme from the plugin owner's snapshot.
    Contributed(Candidate<'a>),
}
/// No retained registry: the caller builds this borrow from current snapshots.
#[derive(Debug)]
pub struct Catalog<'a>(Vec<Entry<'a>>);
impl<'a> Catalog<'a> {
    /// Deduplicate by logical key, keeping preferred eligible host then first.
    #[must_use]
    pub fn new(
        candidates: impl IntoIterator<Item = Candidate<'a>>,
        preferred: &PreferredHosts,
    ) -> Self {
        let mut distinct = BTreeMap::<PluginThemeId, (usize, Candidate<'a>)>::new();
        for (ordinal, candidate) in candidates.into_iter().enumerate() {
            let key = candidate.key();
            distinct
                .entry(key.clone())
                .and_modify(|(_, current)| {
                    let chosen = preferred.get(&key);
                    if chosen == Some(&candidate.address.node)
                        && chosen != Some(&current.address.node)
                    {
                        *current = candidate;
                    }
                })
                .or_insert((ordinal, candidate));
        }
        let mut entries: Vec<_> = distinct
            .into_iter()
            .map(|(key, (ordinal, candidate))| (ordinal, Entry { key, candidate }))
            .collect();
        entries.sort_by_key(|(ordinal, _)| *ordinal);
        Self(entries.into_iter().map(|(_, entry)| entry).collect())
    }
    /// Choices in reference order, with a single entry per stable key.
    #[must_use]
    pub fn entries(&self) -> &[Entry<'a>] {
        &self.0
    }
    /// Missing selected contributions resolve System while the saved id stays.
    #[must_use]
    pub fn resolve(&self, appearance: &Appearance) -> Resolution<'a> {
        match &appearance.plugin_theme {
            Some(ContributedChoice::Selected(key)) => self
                .0
                .iter()
                .find(|entry| &entry.key == key)
                .map_or(Resolution::Builtin(Theme::System), |entry| {
                    Resolution::Contributed(entry.candidate)
                }),
            Some(ContributedChoice::Remembered(_)) | None => Resolution::Builtin(appearance.theme),
        }
    }
}

#[cfg(test)]
mod tests;
