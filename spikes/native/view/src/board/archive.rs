//! The archive's search and its pages (DESIGN.md §6.12; the page's
//! `data/archive.ts` and `features/board/archive.tsx`).

use super::Row;

/// How many archived cards the archive shows before "Show more": a node
/// keeps every ended session's log, so the archive grows without bound.
pub const ARCHIVE_PAGE: usize = 30;

/// The rows of `rows` that match `query`, ignoring case: by the start of
/// a session's id, a piece of its title or its folder, or its machine's
/// name; all of them for an empty query.
#[must_use]
pub fn search_archive<'a>(rows: &[Row<'a>], query: &str) -> Vec<Row<'a>> {
    let text = query.trim().to_lowercase();
    if text.is_empty() {
        return rows.to_vec();
    }
    rows.iter()
        .filter(|row| {
            let card = row.card;
            // The card's title is its title, else its folder's name, which
            // is in its folder: the page's title-or-folder test, the same.
            card.session.to_string().starts_with(&text)
                || card.title.to_lowercase().contains(&text)
                || card.folder().to_lowercase().contains(&text)
                || card.machine.to_lowercase().contains(&text)
        })
        .copied()
        .collect()
}

/// How much of the archive is shown: a page at a time, from the first
/// again whenever the search changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchivePage {
    shown: usize,
}

impl Default for ArchivePage {
    fn default() -> Self {
        Self {
            shown: ARCHIVE_PAGE,
        }
    }
}

impl ArchivePage {
    /// How many of `found` cards are drawn.
    #[must_use]
    pub fn visible(self, found: usize) -> usize {
        found.min(self.shown)
    }

    /// The number "Show N more" offers, when any are left.
    #[must_use]
    pub fn more(self, found: usize) -> Option<usize> {
        let rest = found.saturating_sub(self.shown);
        (rest > 0).then(|| rest.min(ARCHIVE_PAGE))
    }

    /// One page more.
    #[must_use]
    pub const fn next(self) -> Self {
        Self {
            shown: self.shown.saturating_add(ARCHIVE_PAGE),
        }
    }
}
