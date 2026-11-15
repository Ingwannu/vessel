//! Fork captions prepared after fleet or language changes, independent of UI.
//!
//! [Decision Log]
//! - Purpose: names in a fork's first line follow renames and language changes.
//! - Constraint: rendering must not format a value message or initialize Fluent.
//! - Choice: retain captions by source session, updating only changed titles or
//!   catalogs at fleet/language events; absent sources borrow a plain getter.
//! - Tradeoff: one caption per listed source; removed sessions are evicted.

mod notice;
mod problem;
pub use notice::{Notice, PreparedNotice};
pub use problem::{OriginProblem, Problem};

use crate::{
    Fleet,
    localization::{FormatError, Translator},
};
use std::collections::HashMap;
use std::sync::Arc;
use vessel_proto::SessionId;

#[derive(Debug)]
struct Caption {
    title: String,
    words: Arc<str>,
    seen: bool,
}

/// Retained source titles, with the same first-listed match as `Fleet::title_of`.
#[derive(Debug, Default)]
pub struct Headers {
    captions: HashMap<SessionId, Caption>,
    catalog: Option<&'static Translator>,
}

impl Headers {
    /// Refresh at a state/language event, never during rendering.
    /// # Errors
    /// Returns the typed formatter error if translation and English both fail.
    pub fn refresh(
        &mut self,
        fleet: &Fleet,
        catalog: &'static Translator,
    ) -> Result<bool, FormatError> {
        let changed = self.catalog.is_none_or(|old| !std::ptr::eq(old, catalog));
        let mut refreshed = false;
        for caption in self.captions.values_mut() {
            caption.seen = false;
        }
        for card in fleet.machines.iter().flat_map(|machine| &machine.sessions) {
            if self
                .captions
                .get(&card.session)
                .is_some_and(|caption| caption.seen)
            {
                continue;
            }
            let title = card.title.trim();
            if let Some(caption) = self.captions.get_mut(&card.session)
                && !changed
                && caption.title == title
            {
                caption.seen = true;
                continue;
            }
            let words = if title.is_empty() {
                Arc::from(catalog.fork().from_other())
            } else {
                Arc::from(catalog.fork().from_title(title)?)
            };
            refreshed = true;
            let _previous = self.captions.insert(
                card.session,
                Caption {
                    title: title.to_owned(),
                    words,
                    seen: true,
                },
            );
        }
        let before = self.captions.len();
        self.captions.retain(|_, caption| caption.seen);
        self.catalog = Some(catalog);
        Ok(refreshed || before != self.captions.len())
    }

    /// Borrow a prepared caption, or the catalog's plain unknown-source line.
    #[must_use]
    pub fn words<'a>(&'a self, source: SessionId, catalog: &'a Translator) -> &'a str {
        self.captions
            .get(&source)
            .map_or_else(|| catalog.fork().from_other(), |caption| &caption.words)
    }
}

#[cfg(test)]
mod tests;
