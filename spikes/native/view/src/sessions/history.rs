//! Chronological history with metadata-only search and stable local paging.
use crate::machine_book::Name as NodeName;
use crate::{Card, Fleet};
use chrono::NaiveDate;
/// Paseo's directory defaults to fifty entries; fetch more explicitly.
const PAGE_SIZE: usize = 50;
/// Paseo's date sections use rolling seven- and thirty-day boundaries.
const WEEK_DAYS: i64 = 7;
const MONTH_DAYS: i64 = 30;
/// A selected saved host, validated at the node-book boundary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HostScope {
    /// Merge every saved host without hiding individual failures.
    #[default]
    All,
    /// Restrict the directory to one saved host.
    Host(NodeName),
}
/// Date headings follow the viewer's calendar rather than UTC midnights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateBucket {
    /// The current local calendar date, including future clock skew.
    Today,
    /// The previous local calendar date.
    Yesterday,
    /// Two through seven calendar days ago.
    ThisWeek,
    /// Eight through thirty calendar days ago.
    ThisMonth,
    /// Earlier history.
    Older,
}
impl DateBucket {
    /// The app supplies local dates, keeping clock access out of view.
    #[must_use]
    pub fn of(activity: NaiveDate, today: NaiveDate) -> Self {
        let days = today.signed_duration_since(activity).num_days();
        if days <= 0 {
            Self::Today
        } else if days == 1 {
            Self::Yesterday
        } else if days <= WEEK_DAYS {
            Self::ThisWeek
        } else if days <= MONTH_DAYS {
            Self::ThisMonth
        } else {
            Self::Older
        }
    }
}
/// A growing visible window, reset on search or host changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryPage(usize);
impl Default for HistoryPage {
    fn default() -> Self {
        Self(PAGE_SIZE)
    }
}
impl HistoryPage {
    /// Saturate rather than overflow when load-more is repeated.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(PAGE_SIZE))
    }
    /// The number of matching rows the screen can currently display.
    #[must_use]
    pub fn visible(self, total: usize) -> usize {
        total.min(self.0)
    }
    /// Whether a load-more control is needed.
    #[must_use]
    pub const fn has_more(self, total: usize) -> bool {
        total > self.0
    }
}
/// Search only directory metadata and keep chronology with deterministic ties.
#[must_use]
pub fn history<'a>(fleet: &'a Fleet, scope: &HostScope, query: &str) -> Vec<&'a Card> {
    let query = crate::matcher::Query::typo_tolerant(query);
    let mut rows: Vec<&Card> = fleet
        .machines
        .iter()
        .filter(|machine| match scope {
            HostScope::All => true,
            HostScope::Host(name) => machine.name == name.as_str(),
        })
        .flat_map(|machine| &machine.sessions)
        .filter(|card| matches(card, &query))
        .collect();
    rows.sort_by(|one, two| {
        two.detail
            .active_at
            .cmp(&one.detail.active_at)
            .then_with(|| one.machine.cmp(&two.machine))
            .then_with(|| one.session.cmp(&two.session))
    });
    rows
}
fn matches(card: &Card, query: &crate::matcher::Query) -> bool {
    // Workspace/project display names join these fields through the canonical
    // registry projection; neither host labels nor transcripts are history names.
    let fields = [
        card.title.as_str(),
        card.detail.branch.as_deref().unwrap_or_default(),
    ];
    query.score(&fields, None).is_some()
}
