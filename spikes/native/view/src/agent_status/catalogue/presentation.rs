//! Catalog bodies from the existing page's accepted read, with no second cache.
use super::ranked;
use vessel_proto::agents::{Catalogue, CatalogueEntry};

/// The machine page supplies only its own node/ticket-accepted previous copy.
/// A failed first fetch cannot become a successful-looking empty registry.
#[derive(Debug)]
pub enum Read<'a, E> {
    /// The registry has not been requested on this page.
    NotRequested,
    /// Keep a successfully fetched copy while another fetch runs.
    Pending {
        /// An unfetched node copy is not a successful previous catalog.
        previous: Option<&'a Catalogue>,
    },
    /// Keep usable previous entries with the action's failure above them.
    Failed {
        /// Original typed failure, translated by its operation context.
        error: &'a E,
        /// Same machine's last successful copy, if any.
        previous: Option<&'a Catalogue>,
    },
    /// A node reply can still be unfetched when only its cache was requested.
    Ready(&'a Catalogue),
}
// This descriptor only borrows data. Manual implementations avoid derive's
// unnecessary E: Copy/Clone bound: real transport errors are not cloneable.
impl<E> Copy for Read<'_, E> {}
impl<E> Clone for Read<'_, E> {
    fn clone(&self) -> Self {
        *self
    }
}
/// Pending work is neutral, not an availability or failure verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    /// No registry fetch is in flight.
    Idle,
    /// A fetch is in flight; a repeated gesture must not start another.
    Pending,
}
/// Mutually exclusive bodies, independent of how the kit renders a tile.
#[derive(Debug)]
pub enum Body<'a, E> {
    /// Never fetched is distinct from a fetched empty list.
    NotRequested,
    /// No usable entries yet while reading the registry.
    Loading,
    /// First fetch failed; no cached data can hide that failure.
    Failed(&'a E),
    /// The machine successfully fetched a registry without entries.
    Empty,
    /// A fetched nonempty registry has no matching title/id.
    NoMatches,
    /// Nonempty ranked rows; registry order remains untouched.
    Rows {
        /// Makes an empty successful rows body unrepresentable.
        first: &'a CatalogueEntry,
        /// Remaining entries in the same relevance/name order.
        rest: Vec<&'a CatalogueEntry>,
    },
}
/// A retained failure is shown once, separately from usable cached rows.
#[derive(Debug)]
pub struct Presentation<'a, E> {
    /// What replaces the tile grid.
    pub body: Body<'a, E>,
    /// Neutral refresh state, even if the previous list was empty.
    pub activity: Activity,
    /// The failure above retained facts, never duplicated in the body.
    pub retained_failure: Option<&'a E>,
}

/// A node's default empty/unfetched copy must not claim that its registry
/// is empty. Cached failures retain facts without masquerading as success.
#[must_use]
pub fn present<'a, E>(read: Read<'a, E>, query: &str) -> Presentation<'a, E> {
    match read {
        Read::NotRequested => shown(Body::NotRequested, Activity::Idle, None),
        Read::Ready(catalogue) => shown(body(catalogue, query), Activity::Idle, None),
        Read::Pending { previous } => {
            let content = previous
                .filter(|copy| copy.fetched_at.is_some() && !copy.entries.is_empty())
                .map_or(Body::Loading, |copy| body(copy, query));
            shown(content, Activity::Pending, None)
        }
        Read::Failed { error, previous } => match previous.filter(|copy| copy.fetched_at.is_some())
        {
            Some(copy) => shown(body(copy, query), Activity::Idle, Some(error)),
            None => shown(Body::Failed(error), Activity::Idle, None),
        },
    }
}
fn shown<'a, E>(
    body: Body<'a, E>,
    activity: Activity,
    retained_failure: Option<&'a E>,
) -> Presentation<'a, E> {
    Presentation {
        body,
        activity,
        retained_failure,
    }
}
fn body<'a, E>(catalogue: &'a Catalogue, query: &str) -> Body<'a, E> {
    if catalogue.fetched_at.is_none() {
        return Body::NotRequested;
    }
    if catalogue.entries.is_empty() {
        return Body::Empty;
    }
    let mut rows = ranked(&catalogue.entries, query).into_iter();
    rows.next().map_or(Body::NoMatches, |first| Body::Rows {
        first,
        rest: rows.collect(),
    })
}

#[cfg(test)]
mod tests;
