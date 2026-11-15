//! Discovery values for background sessions (ADR 0063).
//!
//! These values do not change a session's durability or exact-id operations.
//! Request and storage fields are a later slice: creation must retain an
//! optional override so omission can inherit the caller's listing.

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
