//! Searchable rule-zone choices from the same pinned bundled database as the
//! node. Explicit `bundled()` avoids system-database filesystem discovery in View.
use vessel_proto::automation::{InvalidAutomationText, TimeZoneName};

/// A bundled choice list cannot silently become an empty successful picker.
#[derive(Debug, thiserror::Error)]
pub enum ZoneChoicesError {
    /// A build omitted the required bundled feature/data.
    #[error("bundled time zone choices are unavailable")]
    Unavailable,
    /// Retain an unexpected database name/grammar mismatch without skipping it.
    #[error("bundled time zone identity: {0}")]
    Name(#[source] InvalidAutomationText),
}
/// Build once outside paint, with already parsed choice identities.
/// # Errors
/// An empty/malformed bundle is a typed failure, never guessed UTC or a partial list.
pub fn bundled() -> Result<Vec<TimeZoneName>, ZoneChoicesError> {
    let database = jiff::tz::TimeZoneDatabase::bundled();
    let mut names = database
        .available()
        .map(|name| name.as_str().parse().map_err(ZoneChoicesError::Name))
        .collect::<Result<Vec<TimeZoneName>, _>>()?;
    if names.is_empty() {
        return Err(ZoneChoicesError::Unavailable);
    }
    names.sort();
    names.dedup();
    Ok(names)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_pinned_bundle_supplies_typed_search_choices_without_a_system_lookup() {
        let names = bundled().unwrap();
        for zone in ["Asia/Seoul", "Europe/London", "America/New_York", "UTC"] {
            assert!(names.contains(&zone.parse().unwrap()));
        }
        assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
