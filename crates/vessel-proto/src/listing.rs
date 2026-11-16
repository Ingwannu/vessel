//! Discovery values for background sessions (ADR 0063).
//!
//! These values do not change a session's durability or exact-id operations.
//! The request distinguishes legacy full reads from explicit discovery.
//! Creation retains an optional override so omission can inherit its caller.

use serde::{Deserialize, Serialize};

/// A creation-only discovery choice, separate from the session's run state.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SessionListing {
    /// Legacy sessions and creations without a caller are discoverable.
    #[default]
    Shown,
    /// Still durable and directly accessible; excluded from default discovery.
    Background,
}

impl SessionListing {
    /// Omit the legacy shown state from replies that need no metadata.
    #[must_use]
    pub const fn is_shown(&self) -> bool {
        matches!(self, Self::Shown)
    }
}

/// A discovery read's inclusion policy, never permission to operate on an id.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ListingScope {
    /// A default read leaves background sessions out.
    #[default]
    Shown,
    /// An explicit inclusive read discovers both kinds.
    All,
}

impl ListingScope {
    /// Preserve shown sessions when a person asks to include background work.
    #[must_use]
    pub const fn includes(self, listing: SessionListing) -> bool {
        match (self, listing) {
            (Self::Shown | Self::All, SessionListing::Shown)
            | (Self::All, SessionListing::Background) => true,
            (Self::Shown, SessionListing::Background) => false,
        }
    }
}

/// Scope and an AND predicate on current labels. Scope is required on the
/// wire so an empty object cannot silently choose a discovery policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SessionSelection {
    /// Background inclusion is separate from permission to access an id.
    pub scope: ListingScope,
    /// Every requested key and value must match; empty adds no predicate.
    #[serde(default)]
    pub labels: crate::SessionLabels,
}

impl SessionSelection {
    /// Normal discovery, with no label predicate.
    #[must_use]
    pub fn shown() -> Self {
        Self {
            scope: ListingScope::Shown,
            labels: crate::SessionLabels::default(),
        }
    }

    /// Inclusive discovery, with no label predicate.
    #[must_use]
    pub fn all() -> Self {
        Self {
            scope: ListingScope::All,
            labels: crate::SessionLabels::default(),
        }
    }

    /// Preserve the chosen scope while adding a validated current-label predicate.
    #[must_use]
    pub fn with_labels(mut self, labels: crate::SessionLabels) -> Self {
        self.labels = labels;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_discovery_hides_background_and_an_inclusive_read_keeps_both() {
        assert_eq!(SessionListing::default(), SessionListing::Shown);
        assert_eq!(ListingScope::default(), ListingScope::Shown);
        assert!(ListingScope::default().includes(SessionListing::Shown));
        assert!(!ListingScope::default().includes(SessionListing::Background));
        assert!(ListingScope::All.includes(SessionListing::Shown));
        assert!(ListingScope::All.includes(SessionListing::Background));
    }

    #[test]
    fn listing_names_round_trip_and_refuse_unknown_or_boolean_values() {
        for (listing, text) in [
            (SessionListing::Shown, r#""shown""#),
            (SessionListing::Background, r#""background""#),
        ] {
            assert_eq!(serde_json::to_string(&listing).unwrap(), text);
            assert_eq!(
                serde_json::from_str::<SessionListing>(text).unwrap(),
                listing
            );
        }
        for text in [
            r#""hidden""#,
            r#""all""#,
            "true",
            "false",
            "null",
            "{}",
            "[]",
        ] {
            assert!(
                serde_json::from_str::<SessionListing>(text).is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn scope_names_round_trip_and_do_not_accept_a_listing_as_a_scope() {
        for (scope, text) in [
            (ListingScope::Shown, r#""shown""#),
            (ListingScope::All, r#""all""#),
        ] {
            assert_eq!(serde_json::to_string(&scope).unwrap(), text);
            assert_eq!(serde_json::from_str::<ListingScope>(text).unwrap(), scope);
        }
        for text in [r#""background""#, r#""typo""#, "true", "null", "{}", "[]"] {
            assert!(
                serde_json::from_str::<ListingScope>(text).is_err(),
                "{text}"
            );
        }
    }
}

#[cfg(test)]
mod selection_tests;
