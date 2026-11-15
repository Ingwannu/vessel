//! Every composer's tray (track P's row c): one for the new chat and one
//! per session, kept while the app runs so leaving a session and coming
//! back finds its files.
//!
//! [Decision Log]
//! - Purpose: Paseo keeps a composer's attachments with its draft
//!   (`stores/draft-store`), per agent; the page keeps a tray per session
//!   for the tab's life (`web/src/features/session/composer.tsx`).
//! - Constraint: a file's bytes may be in memory (a pasted image), so the
//!   trays cannot grow without end; a file cannot be written to the
//!   drafts' file.
//! - Choice: at most [`Trays::MAX`] trays, the one least recently used
//!   dropped first (the page's 16); the new chat's tray becomes its
//!   session's once the session exists ([`Trays::rename`]), so its
//!   uploads' answers find it.
//! - Cost: a session's attached files are forgotten when the app quits or
//!   sixteen other composers were used since.

use std::collections::HashMap;

use super::drafts::DraftKey;
use super::tray::Tray;

/// Every composer's tray, each with when it was last used.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trays {
    trays: HashMap<DraftKey, (u64, Tray)>,
    clock: u64,
}

impl Trays {
    /// The most trays kept.
    pub const MAX: usize = 16;

    /// `key`'s tray, if it has one.
    #[must_use]
    pub fn get(&self, key: DraftKey) -> Option<&Tray> {
        self.trays.get(&key).map(|(_, tray)| tray)
    }

    /// `key`'s tray to change, if it has one (an upload's answer for a
    /// tray that was dropped finds none).
    pub fn get_mut(&mut self, key: DraftKey) -> Option<&mut Tray> {
        self.trays.get_mut(&key).map(|(_, tray)| tray)
    }

    /// `key`'s tray, made when it has none, and now the most recently used
    /// (the least recently used goes when a new one passes [`Self::MAX`]).
    pub fn tray(&mut self, key: DraftKey) -> &mut Tray {
        if !self.trays.contains_key(&key) && self.trays.len() >= Self::MAX {
            let oldest = self
                .trays
                .iter()
                .min_by_key(|(_, (used, _))| *used)
                .map(|(held, _)| *held);
            if let Some(oldest) = oldest {
                drop(self.trays.remove(&oldest));
            }
        }
        self.clock = self.clock.saturating_add(1);
        let (used, tray) = self.trays.entry(key).or_default();
        *used = self.clock;
        tray
    }

    /// `from`'s tray is now `to`'s (the new chat's, once its session
    /// exists); `to`'s own, if it had one, is replaced.
    pub fn rename(&mut self, from: DraftKey, to: DraftKey) {
        if let Some(entry) = self.trays.remove(&from) {
            drop(self.trays.insert(to, entry));
        }
    }

    /// How many trays are kept.
    #[must_use]
    pub fn len(&self) -> usize {
        self.trays.len()
    }

    /// Whether none is.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.trays.is_empty()
    }
}

#[cfg(test)]
mod tests;
