//! Quota presentation preserves unknown percentages and vendor reset instants.
use chrono::{DateTime, FixedOffset, Utc};
use vessel_proto::conversation::FiniteNumber;

use crate::localization::{Date, Translator};

/// Percent drawing and its complement share exactly one bound.
const FULL_PERCENT: f64 = 100.0;
const FULL_DRAW_PERCENT: u8 = 100;
const EMPTY_PERCENT: f64 = 0.0;

/// Paseo's used/remaining choice changes presentation only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayAs {
    /// Default display is consumption already reported.
    #[default]
    Used,
    /// Complement of the reported used percentage.
    Remaining,
}

/// A clamped display value cannot be mistaken for the raw vendor observation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayPercent(f64);
impl DisplayPercent {
    /// Clamp only for display; never modify the underlying vendor snapshot.
    #[must_use]
    #[expect(
        clippy::float_arithmetic,
        reason = "A finite percent's bounded complement is presentation arithmetic, never accounting"
    )]
    pub fn reported(used: FiniteNumber, display: DisplayAs) -> Self {
        let used = used.get().clamp(EMPTY_PERCENT, FULL_PERCENT);
        Self(match display {
            DisplayAs::Used => used,
            DisplayAs::Remaining => FULL_PERCENT - used,
        })
    }
    /// Each mode prefers its explicitly supplied percentage before complementing the other.
    #[must_use]
    pub fn from_reported(
        used: Option<FiniteNumber>,
        remaining: Option<FiniteNumber>,
        display: DisplayAs,
    ) -> Option<Self> {
        match display {
            DisplayAs::Used => used
                .map(|value| Self::reported(value, DisplayAs::Used))
                .or_else(|| remaining.map(|value| Self::reported(value, DisplayAs::Remaining))),
            DisplayAs::Remaining => remaining
                .map(|value| Self::reported(value, DisplayAs::Used))
                .or_else(|| used.map(|value| Self::reported(value, DisplayAs::Remaining))),
        }
    }
    /// Integer drawing shares avoid a float-to-integer cast at the kit boundary.
    #[must_use]
    pub fn whole(self) -> u8 {
        (0..=FULL_DRAW_PERCENT)
            .find(|value| f64::from(*value) >= self.0.round())
            .unwrap_or(FULL_DRAW_PERCENT)
    }
    /// Rounded whole percentages match Paseo's usage window labels.
    #[must_use]
    pub fn text(self) -> String {
        format!("{}%", self.0.round())
    }
}

/// The caller resolves the device's offset; view never reads a clock or environment.
#[must_use]
pub fn timestamp(at: DateTime<Utc>, offset: FixedOffset, words: &'static Translator) -> String {
    let local = at.with_timezone(&offset);
    let date = Date::DateTime {
        date: local.date_naive(),
        time: local.time(),
        zone: offset.to_string(),
    };
    words
        .usage()
        .timestamp(&date)
        .unwrap_or_else(|_format| words.usage().none().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::{ENGLISH, Locale};
    #[test]
    fn only_the_drawing_is_clamped_and_remaining_is_the_complement() {
        let above = FiniteNumber::new(140.0).unwrap();
        assert_eq!(
            DisplayPercent::reported(above, DisplayAs::Used).text(),
            "100%"
        );
        assert_eq!(
            DisplayPercent::reported(above, DisplayAs::Remaining).text(),
            "0%"
        );
        assert_eq!(above.get().to_string(), "140");
        let value = FiniteNumber::new(31.0).unwrap();
        assert_eq!(
            DisplayPercent::reported(value, DisplayAs::Remaining).text(),
            "69%"
        );
        let below = FiniteNumber::new(-1.0).unwrap();
        assert_eq!(
            DisplayPercent::reported(below, DisplayAs::Used).text(),
            "0%"
        );
    }
    #[test]
    fn instants_use_the_viewers_offset_and_translated_calendar() {
        let instant: DateTime<Utc> = "2026-11-15T01:30:00Z".parse().unwrap();
        let offset = FixedOffset::east_opt(9 * 3600).unwrap();
        let en = Translator::get(ENGLISH).unwrap();
        let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
        let english = timestamp(instant, offset, en);
        let korean = timestamp(instant, offset, ko);
        assert!(english.contains("10:30 AM"), "{english}");
        assert!(korean.contains("10:30"), "{korean}");
        assert!(english.contains("+09:00") && korean.contains("+09:00"));
    }
}
