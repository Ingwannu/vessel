//! A pull request's activity as Paseo's pane lists it
//! (`pull-request-panel/timeline.ts`, `activity-state.ts`,
//! `activity-location.ts`, and `data.ts`'s mapping and `formatAge`), from
//! `pr.timeline`'s items (ADR 0088 part 3), which come oldest first.
//!
//! [Decision Log]
//! - Purpose: the pane's cards (comments, reviews with their threads,
//!   threads), what is folded at first, the header's counts and each
//!   card's age, decided once and tested outside the screen.
//! - Choice: Paseo's steps in its order: drop what has nothing to say,
//!   gather a thread's comments where its first one stands, move a thread
//!   under the review its first comment belongs to; a resolved or
//!   outdated thread starts folded, nothing else does.
//! - Difference: Paseo writes ages in English only ("3h ago"); here the
//!   step is a value and the language words it.

use chrono::{DateTime, Utc};
use vessel_proto::forge::{ForgeItemId, ReviewState, ThreadPlace, TimelineItem};

/// A review thread: its place, and its comments oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread<'a> {
    /// Where it is, and whether it is resolved or outdated.
    pub place: &'a ThreadPlace,
    /// Its comments; the first is its root.
    pub comments: Vec<&'a TimelineItem>,
}

impl Thread<'_> {
    /// Its key among the entries (Paseo's `thread:<id>`).
    #[must_use]
    pub fn key(&self) -> String {
        format!("thread:{}", self.place.thread.as_str())
    }

    /// Paseo's `shouldCollapseByDefault`: resolved or outdated.
    #[must_use]
    pub const fn folded_at_first(&self) -> bool {
        self.place.resolved || self.place.outdated
    }
}

/// One card of the activity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry<'a> {
    /// A conversation comment, or a review with no thread of its own.
    Single(&'a TimelineItem),
    /// A thread whose review is not listed.
    Thread(Thread<'a>),
    /// A review and the threads its comments started.
    Review {
        /// The review.
        review: &'a TimelineItem,
        /// Its threads, in order.
        threads: Vec<Thread<'a>>,
    },
}

impl Entry<'_> {
    /// Its key, for what a person folded (Paseo's entry id).
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::Single(item) | Self::Review { review: item, .. } => item.id().as_str().to_owned(),
            Self::Thread(thread) => thread.key(),
        }
    }

    /// Whether it starts folded: only a thread can.
    #[must_use]
    pub const fn folded_at_first(&self) -> bool {
        match self {
            Self::Thread(thread) => thread.folded_at_first(),
            Self::Single(_) | Self::Review { .. } => false,
        }
    }
}

/// Whether an item has something to say (Paseo drops an empty comment, and
/// a review that only commented with no words).
#[must_use]
pub fn says_something(item: &TimelineItem) -> bool {
    match item {
        TimelineItem::Comment { body, .. } => !body.as_str().trim().is_empty(),
        TimelineItem::Review { body, state, .. } => {
            !body.as_str().trim().is_empty()
                || matches!(state, ReviewState::Approved | ReviewState::ChangesRequested)
        }
    }
}

/// The items worth a card, in their order.
#[must_use]
pub fn shown(items: &[TimelineItem]) -> Vec<&TimelineItem> {
    items.iter().filter(|item| says_something(item)).collect()
}

/// A comment's thread, when it is in one.
const fn thread_of(item: &TimelineItem) -> Option<&ThreadPlace> {
    match item {
        TimelineItem::Comment { thread, .. } => thread.as_ref(),
        TimelineItem::Review { .. } => None,
    }
}

/// Whether an item is a review.
const fn is_review(item: &TimelineItem) -> bool {
    matches!(item, TimelineItem::Review { .. })
}

/// The review a comment belongs to.
const fn review_of(item: &TimelineItem) -> Option<&ForgeItemId> {
    match item {
        TimelineItem::Comment { review, .. } => review.as_ref(),
        TimelineItem::Review { .. } => None,
    }
}

/// Paseo's `groupThreads` then `nestThreadsUnderReviews`.
#[must_use]
pub fn entries<'a>(items: &[&'a TimelineItem]) -> Vec<Entry<'a>> {
    let mut grouped: Vec<Entry<'a>> = Vec::new();
    for &item in items {
        let Some(place) = thread_of(item) else {
            grouped.push(Entry::Single(item));
            continue;
        };
        let existing = grouped.iter_mut().find_map(|entry| match entry {
            Entry::Thread(thread) if thread.place.thread == place.thread => Some(thread),
            Entry::Single(_) | Entry::Thread(_) | Entry::Review { .. } => None,
        });
        match existing {
            Some(thread) => thread.comments.push(item),
            None => grouped.push(Entry::Thread(Thread {
                place,
                comments: vec![item],
            })),
        }
    }
    let reviews: Vec<&'a ForgeItemId> = grouped
        .iter()
        .filter_map(|entry| match *entry {
            Entry::Single(item) if is_review(item) => Some(item.id()),
            Entry::Single(_) | Entry::Thread(_) | Entry::Review { .. } => None,
        })
        .collect();
    let (nested, top): (Vec<Entry<'a>>, Vec<Entry<'a>>) =
        grouped.into_iter().partition(|entry| match entry {
            Entry::Thread(thread) => thread
                .comments
                .first()
                .and_then(|root| review_of(root))
                .is_some_and(|review| reviews.contains(&review)),
            Entry::Single(_) | Entry::Review { .. } => false,
        });
    let mut nested: Vec<Thread<'a>> = nested
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Thread(thread) => Some(thread),
            Entry::Single(_) | Entry::Review { .. } => None,
        })
        .collect();
    top.into_iter()
        .map(|entry| match entry {
            Entry::Single(review) if is_review(review) => {
                let (mine, rest): (Vec<Thread<'a>>, Vec<Thread<'a>>) =
                    nested.drain(..).partition(|thread| {
                        thread.comments.first().and_then(|root| review_of(root))
                            == Some(review.id())
                    });
                nested = rest;
                if mine.is_empty() {
                    Entry::Single(review)
                } else {
                    Entry::Review {
                        review,
                        threads: mine,
                    }
                }
            }
            other @ (Entry::Single(_) | Entry::Thread(_) | Entry::Review { .. }) => other,
        })
        .collect()
}

/// The Activity header's pills (Paseo counts the flat list).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// Approving reviews.
    pub approvals: usize,
    /// Reviews that requested changes.
    pub changes: usize,
    /// Comments, thread replies included, and reviews that commented.
    pub comments: usize,
}

/// The counts of `items`.
#[must_use]
pub fn counts(items: &[&TimelineItem]) -> Counts {
    items.iter().fold(Counts::default(), |counts, item| match item {
        TimelineItem::Review {
            state: ReviewState::Approved,
            ..
        } => Counts {
            approvals: counts.approvals.saturating_add(1),
            ..counts
        },
        TimelineItem::Review {
            state: ReviewState::ChangesRequested,
            ..
        } => Counts {
            changes: counts.changes.saturating_add(1),
            ..counts
        },
        TimelineItem::Review {
            state: ReviewState::Commented | ReviewState::Unknown,
            ..
        }
        | TimelineItem::Comment { .. } => Counts {
            comments: counts.comments.saturating_add(1),
            ..counts
        },
    })
}

/// How long ago, in Paseo's steps (floored).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
    /// Under a minute.
    JustNow,
    /// Minutes, under an hour.
    Minutes(i64),
    /// Hours, under a day.
    Hours(i64),
    /// Days, under thirty.
    Days(i64),
    /// Months of thirty days, under a year.
    Months(i64),
    /// Years of 365 days.
    Years(i64),
}

impl Age {
    /// Paseo's `formatAge` of `at` seen from `now` (a time after `now` is
    /// "just now").
    #[must_use]
    pub fn of(at: DateTime<Utc>, now: DateTime<Utc>) -> Self {
        let elapsed = now.signed_duration_since(at);
        let (minutes, hours, days) = (
            elapsed.num_minutes(),
            elapsed.num_hours(),
            elapsed.num_days(),
        );
        if elapsed.num_seconds() < 60 {
            Self::JustNow
        } else if minutes < 60 {
            Self::Minutes(minutes)
        } else if hours < 24 {
            Self::Hours(hours)
        } else if days < 30 {
            Self::Days(days)
        } else if days < 365 {
            Self::Months(days.checked_div(30).unwrap_or_default())
        } else {
            Self::Years(days.checked_div(365).unwrap_or_default())
        }
    }
}

/// A thread's place as its header names it (Paseo's
/// `formatPullRequestThreadPath`): `path:start-line`, `path:line` or the
/// path; `None` without a path ("Discussion thread").
#[must_use]
pub fn thread_path(place: &ThreadPlace) -> Option<String> {
    let path = place.path.as_ref()?.as_str();
    Some(match (place.start_line, place.line) {
        (Some(start), Some(line)) => format!("{path}:{start}-{line}"),
        (None, Some(line)) => format!("{path}:{line}"),
        (Some(_) | None, None) => path.to_owned(),
    })
}

#[cfg(test)]
mod tests;
