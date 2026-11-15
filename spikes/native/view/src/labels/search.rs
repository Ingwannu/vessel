//! Search trims the protocol's whitespace but preserves internal query spaces.

use super::Name;

pub(super) struct Query(String);

impl Query {
    pub(super) fn new(raw: &str) -> Self {
        Self(vessel_proto::workspace_labels::trim_whitespace(raw).to_lowercase())
    }
    pub(super) fn matches(&self, name: &Name) -> bool {
        name.key().as_str().contains(&self.0)
    }
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_edges_follow_ecmascript_and_internal_whitespace_is_not_collapsed() {
        let ready = "Ready".parse().unwrap();
        assert!(Query::new("\u{feff} READY \u{feff}").matches(&ready));
        assert!(!Query::new("\u{0085}Ready").matches(&ready));
        assert!(Query::new(" \u{feff} ").is_empty());
        assert!(!Query::new("\u{0085}").is_empty());
        assert!(!Query::new("In  Progress").matches(&"In Progress".parse().unwrap()));
    }
}
