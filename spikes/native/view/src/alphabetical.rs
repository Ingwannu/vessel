//! One locale-independent alphabetical order for displayed names.
//!
//! [Decision Log]
//! - Purpose: provider catalogs and command-name ties use the same order.
//! - Constraint: locale collation is unavailable in the current app stack;
//!   adding a separate data-backed collator would change its size contract.
//! - Choice: compare Unicode-lowercased strings, then original scalar order.
//!   This extracts the existing composer's exact comparison, using only std.
//! - Cost: accents and locale-tailored scripts can differ from Paseo's
//!   localeCompare sensitivity=base; no normalization or accent folding is
//!   implied. Whole-string lowercase preserves contextual final sigma.

use std::cmp::Ordering;

/// Deterministic title/name order, independent of the operating system's
/// locale: Unicode lowercase first, original code points for equal folds.
/// Callers retain their own relevance score or source order before this tie.
#[must_use]
pub fn compare(one: &str, other: &str) -> Ordering {
    one.to_lowercase()
        .cmp(&other.to_lowercase())
        .then_with(|| one.cmp(other))
}

#[cfg(test)]
mod tests;
