//! The activity's words: a card's verb and its age (Paseo writes ages in
//! English only; here each step is a message).

use vessel_proto::forge::{ReviewState, TimelineItem};

use super::timeline::Age;
use crate::localization::{Number, Translator};

/// A step's count, at most `u32::MAX`.
fn number(count: i64) -> Number {
    Number::count(u32::try_from(count).unwrap_or(u32::MAX))
}

impl Age {
    /// "just now", "3m ago", "2h ago", "5d ago", "2mo ago", "1y ago".
    #[must_use]
    pub fn words(self, catalog: &Translator) -> String {
        let words = catalog.pulls();
        match self {
            Self::JustNow => Ok(words.age_now().to_owned()),
            Self::Minutes(count) => words.age_minutes(number(count)),
            Self::Hours(count) => words.age_hours(number(count)),
            Self::Days(count) => words.age_days(number(count)),
            Self::Months(count) => words.age_months(number(count)),
            Self::Years(count) => words.age_years(number(count)),
        }
        .unwrap_or_default()
    }
}

/// What an item's author did (Paseo's `ActivityVerb`, lowercase).
#[must_use]
pub fn verb<'a>(item: &TimelineItem, catalog: &'a Translator) -> &'a str {
    let words = catalog.pulls();
    match item {
        TimelineItem::Comment { .. } => words.verb_commented(),
        TimelineItem::Review { state, .. } => match state {
            ReviewState::Approved => words.verb_approved(),
            ReviewState::ChangesRequested => words.verb_changes_requested(),
            ReviewState::Commented | ReviewState::Unknown => words.verb_reviewed(),
        },
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone as _, Utc};

    use super::*;
    use crate::localization::{ENGLISH, Locale};

    fn plain(text: &str) -> String {
        text.replace(['\u{2068}', '\u{2069}'], "")
    }

    #[test]
    fn ages_read_as_paseo_s_and_whole_in_korean() {
        let en = Translator::get(ENGLISH).unwrap();
        let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 19, 10, 0, 0).unwrap();
        let ago = |minutes: i64| Age::of(now - Duration::minutes(minutes), now);
        assert_eq!(Age::JustNow.words(en), "just now");
        assert_eq!(plain(&ago(3).words(en)), "3m ago");
        assert_eq!(plain(&ago(120).words(en)), "2h ago");
        assert_eq!(plain(&ago(65 * 24 * 60).words(en)), "2mo ago");
        assert_eq!(plain(&ago(120).words(ko)), "2시간 전");
    }
}
