//! Empty picker acquisition is a listing, never an invalid empty `FindQuery`.
use vessel_proto::workspace::FindQuery;
/// File-owner acquisition intent shared by every picker consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileQuery {
    /// Initial root files (requested-parent list until suggestions are admitted).
    All,
    /// A validated nonempty indexed query.
    Matching(FindQuery),
}
impl FileQuery {
    /// Search text is absent for the explicit listing intent.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::All => None,
            Self::Matching(query) => Some(query.as_str()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initial_listing_has_no_protocol_query_and_matching_stays_validated() {
        assert_eq!(FileQuery::All.text(), None);
        assert!("".parse::<FindQuery>().is_err());
        assert!("   ".parse::<FindQuery>().is_err());
        let query = "src/main".parse::<FindQuery>().unwrap();
        assert_eq!(FileQuery::Matching(query).text(), Some("src/main"));
    }
}
