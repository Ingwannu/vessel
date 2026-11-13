//! Cron expressions (protocol 11, ADR 0033): which wall-clock minutes a
//! schedule is due at.
//!
//! [Decision Log]
//! - Purpose: a person writes "at nine on weekdays" as `0 9 * * 1-5`; the
//!   node must know exactly which minutes that means, and a client must
//!   be able to tell before sending whether the node will take it.
//! - Constraint: AGENTS.md section 2: a value with a shape is a type whose
//!   only constructor checks it, so a schedule never holds text the node
//!   would have to parse again (or could not).
//! - Alternatives: a cron crate (`cron` needs a seconds field and has open
//!   reports of DST mistakes; `croner` brings `chrono-tz`'s generated zone
//!   tables and its own DST rules, which ADR 0033 decides differently);
//!   RRULE (agent-orchestrator's, through `rrule-go`; ADR 0033 says why
//!   not); the five fields as a string the node parses when it runs.
//! - Choice: the five fields Paseo and Vixie cron read, parsed here into
//!   bit sets ([`CronFields`]), which the node walks to find the next
//!   matching minute in a zone. The grammar is deliberately narrow: what
//!   all three references and every cron agree on, plus month and day
//!   names and the `@daily`-style shorthands. Refused: seconds, `L`, `W`,
//!   `#` and `?` (each cron means something else by them), a step on a
//!   single value (`5/15`: some crons read it as `5-59/15`, Paseo refuses
//!   it) and a range that wraps (`FRI-MON`).
//! - Cost: the next-time search is ours (the node's `automation::time`),
//!   about two hundred lines with their own tests; nothing to learn about
//!   another library's edge cases.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// A schedule's cron expression: five fields, minute, hour, day of the
/// month, month and day of the week. Kept as written, with runs of white
/// space made one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct CronExpr {
    text: Arc<str>,
    fields: CronFields,
}

/// The minutes a [`CronExpr`] matches, as bit sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CronFields {
    /// Bits 0 to 59.
    minutes: u64,
    /// Bits 0 to 23.
    hours: u32,
    /// Bits 1 to 31.
    days: u32,
    /// Bits 1 to 12.
    months: u16,
    /// Bits 0 to 6, Sunday 0.
    weekdays: u8,
    /// Both day fields are restricted: a day matching either runs
    /// (Vixie cron's rule); otherwise a day must match both.
    days_either: bool,
}

impl CronExpr {
    /// The longest expression taken, in bytes.
    pub const MAX_LEN: usize = 256;

    /// The expression as kept.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// What it matches.
    #[must_use]
    pub const fn fields(&self) -> &CronFields {
        &self.fields
    }
}

impl CronFields {
    /// Whether `minute` (0 to 59) matches.
    #[must_use]
    pub fn minute(&self, minute: u8) -> bool {
        has(self.minutes, minute)
    }

    /// Whether `hour` (0 to 23) matches.
    #[must_use]
    pub fn hour(&self, hour: u8) -> bool {
        has(self.hours.into(), hour)
    }

    /// Whether `month` (1 to 12) matches.
    #[must_use]
    pub fn month(&self, month: u8) -> bool {
        has(self.months.into(), month)
    }

    /// Whether a day matches: `day` of the month (1 to 31) and its
    /// `weekday` (0 to 6, Sunday 0).
    #[must_use]
    pub fn day(&self, day: u8, weekday: u8) -> bool {
        let by_date = has(self.days.into(), day);
        let by_weekday = has(self.weekdays.into(), weekday);
        if self.days_either {
            by_date || by_weekday
        } else {
            by_date && by_weekday
        }
    }

    /// The first matching minute at or after `from`, in this hour.
    #[must_use]
    pub fn next_minute(&self, from: u8) -> Option<u8> {
        (from..60).find(|minute| self.minute(*minute))
    }

    /// The first matching hour at or after `from`, in this day.
    #[must_use]
    pub fn next_hour(&self, from: u8) -> Option<u8> {
        (from..24).find(|hour| self.hour(*hour))
    }
}

/// Whether bit `bit` of `set` is on.
fn has(set: u64, bit: u8) -> bool {
    1_u64
        .checked_shl(bit.into())
        .is_some_and(|mask| set & mask != 0)
}

/// What is wrong with a cron expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidCron {
    /// Longer than [`CronExpr::MAX_LEN`] bytes.
    TooLong,
    /// Not five fields (nor a known `@` shorthand).
    FieldCount {
        /// How many there were.
        found: usize,
    },
    /// An `@` shorthand this grammar does not have (`@reboot` among them).
    Shorthand,
    /// A field that does not parse.
    Field {
        /// Which field.
        field: CronField,
        /// What is wrong, described.
        problem: &'static str,
    },
}

/// One of the five fields, for [`InvalidCron::Field`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronField {
    /// The first.
    Minute,
    /// The second.
    Hour,
    /// The third.
    DayOfMonth,
    /// The fourth.
    Month,
    /// The fifth.
    DayOfWeek,
}

impl fmt::Display for CronField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Minute => "minute",
            Self::Hour => "hour",
            Self::DayOfMonth => "day of the month",
            Self::Month => "month",
            Self::DayOfWeek => "day of the week",
        })
    }
}

impl fmt::Display for InvalidCron {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong => write!(
                f,
                "a cron expression is at most {} bytes",
                CronExpr::MAX_LEN
            ),
            Self::FieldCount { found } => write!(
                f,
                "a cron expression has five fields (minute, hour, day of the month, month, day \
                 of the week), not {found}"
            ),
            Self::Shorthand => f.write_str(
                "the shorthands are @hourly, @daily, @midnight, @weekly, @monthly, @yearly and \
                 @annually",
            ),
            Self::Field { field, problem } => write!(f, "the {field} field {problem}"),
        }
    }
}

impl std::error::Error for InvalidCron {}

/// One field's grammar: its range and the names it takes.
struct Grammar {
    field: CronField,
    min: u8,
    max: u8,
    names: &'static [&'static str],
}

const MINUTE: Grammar = Grammar {
    field: CronField::Minute,
    min: 0,
    max: 59,
    names: &[],
};
const HOUR: Grammar = Grammar {
    field: CronField::Hour,
    min: 0,
    max: 23,
    names: &[],
};
const DAY_OF_MONTH: Grammar = Grammar {
    field: CronField::DayOfMonth,
    min: 1,
    max: 31,
    names: &[],
};
const MONTH: Grammar = Grammar {
    field: CronField::Month,
    min: 1,
    max: 12,
    names: &[
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ],
};
/// 0 to 7, both 0 and 7 Sunday (folded into 0 once parsed).
const DAY_OF_WEEK: Grammar = Grammar {
    field: CronField::DayOfWeek,
    min: 0,
    max: 7,
    names: &["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"],
};

impl Grammar {
    fn problem(&self, problem: &'static str) -> InvalidCron {
        InvalidCron::Field {
            field: self.field,
            problem,
        }
    }

    /// The field's bit set, and whether it is restricted (does not start
    /// with `*`: Vixie cron's test for the day rule).
    fn parse(&self, text: &str) -> Result<(u64, bool), InvalidCron> {
        let mut set = 0_u64;
        for item in text.split(',') {
            set |= self.item(item)?;
        }
        Ok((set, !text.starts_with('*')))
    }

    /// One list item: `*`, `n`, `a-b`, `*/s` or `a-b/s`.
    fn item(&self, item: &str) -> Result<u64, InvalidCron> {
        let (range, step) = match item.split_once('/') {
            Some((range, step)) => (range, Some(step)),
            None => (item, None),
        };
        let (low, high) = if range == "*" {
            (self.min, self.max)
        } else if let Some((low, high)) = range.split_once('-') {
            let (low, high) = (self.value(low)?, self.value(high)?);
            if low > high {
                return Err(self.problem("has a range that runs backwards (no range wraps)"));
            }
            (low, high)
        } else {
            if step.is_some() {
                return Err(self.problem(
                    "takes a step only after `*` or a range (`*/15`, `0-30/5`), not after one \
                     value",
                ));
            }
            let value = self.value(range)?;
            (value, value)
        };
        let step = match step {
            None => 1,
            Some(step) => step
                .parse::<u8>()
                .ok()
                .filter(|step| *step >= 1 && *step <= self.max)
                .ok_or_else(|| self.problem("has a step that is not a number in its range"))?,
        };
        let mut set = 0_u64;
        let mut value = low;
        loop {
            set |= 1_u64.checked_shl(value.into()).unwrap_or(0);
            match value.checked_add(step) {
                Some(next) if next <= high => value = next,
                Some(_) | None => break,
            }
        }
        Ok(set)
    }

    /// A number or a name in the field's range.
    fn value(&self, text: &str) -> Result<u8, InvalidCron> {
        if let Some(index) = self
            .names
            .iter()
            .position(|name| name.eq_ignore_ascii_case(text))
        {
            // Months are named from 1, days of the week from 0.
            let index = u8::try_from(index).map_err(|_overflow| self.problem("has a bad name"))?;
            return index
                .checked_add(self.min)
                .ok_or_else(|| self.problem("has a bad name"));
        }
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(self.problem("holds something that is neither a number nor a name"));
        }
        text.parse::<u8>()
            .ok()
            .filter(|value| (self.min..=self.max).contains(value))
            .ok_or_else(|| self.problem("holds a number outside its range"))
    }
}

/// The five fields an `@` shorthand stands for.
fn shorthand(text: &str) -> Option<&'static str> {
    Some(match text {
        "@yearly" | "@annually" => "0 0 1 1 *",
        "@monthly" => "0 0 1 * *",
        "@weekly" => "0 0 * * 0",
        "@daily" | "@midnight" => "0 0 * * *",
        "@hourly" => "0 * * * *",
        _ => return None,
    })
}

impl FromStr for CronExpr {
    type Err = InvalidCron;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() > Self::MAX_LEN {
            return Err(InvalidCron::TooLong);
        }
        let words: Vec<&str> = text.split_whitespace().collect();
        let kept: Arc<str> = Arc::from(words.join(" "));
        let fields = match words.as_slice() {
            [word] if word.starts_with('@') => {
                let expanded =
                    shorthand(&word.to_ascii_lowercase()).ok_or(InvalidCron::Shorthand)?;
                return expanded.parse::<Self>().map(|parsed| Self {
                    text: kept,
                    fields: parsed.fields,
                });
            }
            [minute, hour, day, month, weekday] => {
                let (minutes, _) = MINUTE.parse(minute)?;
                let (hours, _) = HOUR.parse(hour)?;
                let (days, days_restricted) = DAY_OF_MONTH.parse(day)?;
                let (months, _) = MONTH.parse(month)?;
                let (weekdays, weekdays_restricted) = DAY_OF_WEEK.parse(weekday)?;
                // Sunday is 0 and 7: fold bit 7 into bit 0.
                let weekdays = (weekdays | weekdays.checked_shr(7).unwrap_or(0)) & 0x7f;
                CronFields {
                    minutes,
                    hours: narrow(hours),
                    days: narrow(days),
                    months: narrow(months),
                    weekdays: narrow(weekdays),
                    days_either: days_restricted && weekdays_restricted,
                }
            }
            words => return Err(InvalidCron::FieldCount { found: words.len() }),
        };
        Ok(Self { text: kept, fields })
    }
}

/// A field's set in its own width: every bit a field sets is below the
/// width (its grammar's `max` is), so nothing is lost.
fn narrow<T: TryFrom<u64> + Default>(set: u64) -> T {
    T::try_from(set).unwrap_or_default()
}

impl fmt::Display for CronExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

crate::key::serde_via_text!(CronExpr);

#[cfg(test)]
mod tests {
    use super::*;

    fn cron(text: &str) -> CronFields {
        *text.parse::<CronExpr>().unwrap().fields()
    }

    #[test]
    fn fields_take_lists_ranges_steps_and_names() {
        let fields = cron("*/15 9-17 * jan,MAR-may mon-fri");
        assert!(fields.minute(0) && fields.minute(45) && !fields.minute(50));
        assert!(fields.hour(9) && fields.hour(17) && !fields.hour(18));
        assert!(fields.month(1) && fields.month(4) && !fields.month(2));
        // The day of the month is `*`: the weekday decides.
        assert!(fields.day(13, 1) && !fields.day(13, 0));
        assert_eq!(fields.next_minute(16), Some(30));
        assert_eq!(fields.next_minute(46), None);
        assert_eq!(fields.next_hour(18), None);
        let stepped = cron("0-30/10 0 1 1 *");
        assert!(stepped.minute(30) && stepped.minute(20) && !stepped.minute(35));
    }

    #[test]
    fn sunday_is_zero_and_seven() {
        assert!(cron("0 0 * * 7").day(5, 0));
        assert!(cron("0 0 * * 0").day(5, 0));
        assert!(cron("0 0 * * sun").day(5, 0));
        assert!(!cron("0 0 * * 7").day(5, 6));
        assert!(cron("0 0 * * 5-7").day(5, 0));
    }

    #[test]
    fn two_restricted_day_fields_match_either() {
        // Vixie cron: the 1st of the month, and every Monday.
        let either = cron("0 0 1 * 1");
        assert!(either.day(1, 3) && either.day(8, 1) && !either.day(8, 2));
        // `*` (or a step over it) leaves the other field to decide alone.
        let both = cron("0 0 */2 * 1");
        assert!(both.day(3, 1) && !both.day(3, 2) && !both.day(4, 1));
    }

    #[test]
    fn shorthands_and_white_space() {
        let daily: CronExpr = "@DAILY".parse().unwrap();
        assert_eq!(daily.as_str(), "@DAILY");
        assert_eq!(*daily.fields(), cron("0 0 * * *"));
        let spaced: CronExpr = "  0\t9 *  * 1-5 ".parse().unwrap();
        assert_eq!(spaced.as_str(), "0 9 * * 1-5");
        assert_eq!(serde_json::to_string(&spaced).unwrap(), r#""0 9 * * 1-5""#);
        let back: CronExpr = serde_json::from_str(r#""0 9 * * 1-5""#).unwrap();
        assert_eq!(back, spaced);
    }

    #[test]
    fn what_is_refused() {
        for (text, wanted) in [
            ("@reboot", InvalidCron::Shorthand),
            ("0 0 * *", InvalidCron::FieldCount { found: 4 }),
            ("0 0 0 * * *", InvalidCron::FieldCount { found: 6 }),
        ] {
            assert_eq!(text.parse::<CronExpr>().unwrap_err(), wanted, "{text}");
        }
        for text in [
            "60 * * * *",
            "* 24 * * *",
            "* * 0 * *",
            "* * 32 * *",
            "* * * 13 *",
            "* * * * 8",
            "5/15 * * * *",
            "* * * * fri-mon",
            "*/0 * * * *",
            "*/61 * * * *",
            "L * * * *",
            "? * * * *",
            "* * 1W * *",
            "* * * * 1#2",
            "-1 * * * *",
            "1- * * * *",
            ", * * * *",
            "0 0 * JANUARY *",
        ] {
            let refused = text.parse::<CronExpr>().unwrap_err();
            assert!(
                matches!(refused, InvalidCron::Field { .. }),
                "{text}: {refused:?}"
            );
        }
        let long = format!("{} * * * *", "0,".repeat(200));
        assert_eq!(long.parse::<CronExpr>().unwrap_err(), InvalidCron::TooLong);
        assert!(serde_json::from_str::<CronExpr>(r#""61 * * * *""#).is_err());
    }
}
