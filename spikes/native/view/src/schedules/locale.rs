//! New list words use the generated catalog; accepted cadence/action/error
//! tables remain in their original module until their separately owned move.
//!
//! [Decision Log]
//! - Purpose: an empty state and pending action have distinct complete words.
//! - Constraint: the existing accepted tables are not migrated or duplicated.
//! - Alternatives: render-time string keys or another pair of Rust tables.
//! - Choice: exhaustive typed choices over the generated area accessor.
//! - Why and cost: catalog shape is checked at build time; labels borrow the
//!   prepared language cache and contain no formatting work while painting.

use super::{Action, Bucket};
use crate::localization::Translator;

/// Filter-specific empty title, never reused as a badge or filter label.
#[must_use]
pub fn empty_title(bucket: Bucket, catalog: &Translator) -> &str {
    let words = catalog.schedules();
    match bucket {
        Bucket::Active => words.empty_active(),
        Bucket::Ended => words.empty_ended(),
        Bucket::Unknown => words.empty_unknown(),
    }
}

/// Only the active empty state has the reference's cadence explanation.
#[must_use]
pub fn empty_hint(bucket: Bucket, catalog: &Translator) -> Option<&str> {
    match bucket {
        Bucket::Active => Some(catalog.schedules().empty_description()),
        Bucket::Ended | Bucket::Unknown => None,
    }
}

/// A pending label replaces its exact menu action, with no second row signal.
/// Edit is not wired in the list slice and cannot acquire a pending row here.
#[must_use]
pub fn pending(action: Action, catalog: &Translator) -> Option<&str> {
    let words = catalog.schedules();
    match action {
        Action::Pause => Some(words.pausing()),
        Action::Resume => Some(words.resuming()),
        Action::RunNow => Some(words.starting()),
        Action::Delete => Some(words.deleting()),
        Action::Edit => None,
    }
}

#[cfg(test)]
mod tests;
