//! Numbers, durations and times as the conversation writes them (the
//! page's `data/models.ts`, `data/context-meter.ts`, `data/format.ts`),
//! in whole-number arithmetic: this crate's lints deny floats, and every
//! format here rounds to a tenth or a whole at most.

use chrono::{DateTime, FixedOffset, Utc};

use super::{Lang, Words};

/// `count` with thousands grouped by commas: `1,234,567` (`en-US`, which
/// the page's turn line uses in both languages).
#[must_use]
pub fn grouped(count: u64) -> String {
    let digits = count.to_string();
    let mut out = String::with_capacity(digits.len().saturating_mul(2));
    let lead = digits.len().checked_rem(3).unwrap_or(0);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && index.checked_rem(3) == Some(lead) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// `value / scale` rounded half up.
fn rounded(value: u64, scale: u64) -> u64 {
    let doubled = u128::from(value)
        .saturating_mul(2)
        .checked_div(u128::from(scale))
        .unwrap_or(0);
    u64::try_from(doubled.saturating_add(1).checked_div(2).unwrap_or(0)).unwrap_or(u64::MAX)
}

/// `value / scale` rounded half up, as a count of tenths.
fn tenths(value: u64, scale: u64) -> u64 {
    rounded(value.saturating_mul(10), scale)
}

/// `tenths` as `1.2`, or `12` when it is whole.
fn decimal(tenths: u64) -> String {
    let whole = tenths.checked_div(10).unwrap_or(0);
    match tenths.checked_rem(10).unwrap_or(0) {
        0 => whole.to_string(),
        tenth => format!("{whole}.{tenth}"),
    }
}

/// A token count, short, as `vessel log` writes it: `950`, `1.2k`, `48k`,
/// `1.3M`.
#[must_use]
pub fn short_tokens(count: u64) -> String {
    let (unit, size) = match count {
        0..1000 => return count.to_string(),
        1000..1_000_000 => ("k", 1000),
        _ => ("M", 1_000_000),
    };
    let tenths = tenths(count, size);
    if tenths < 100 {
        format!("{}{unit}", decimal(tenths))
    } else {
        // Ten or more: whole units, rounded.
        format!("{}{unit}", rounded(count, size))
    }
}

/// A token count as the context ring writes it: `950`, `84k`, `1.2M`.
#[must_use]
pub fn meter_tokens(count: u64) -> String {
    match count {
        0..1000 => count.to_string(),
        1000..1_000_000 => format!("{}k", rounded(count, 1000)),
        _ => {
            let tenths = tenths(count, 1_000_000);
            if tenths < 100 {
                let whole = tenths.checked_div(10).unwrap_or(0);
                let tenth = tenths.checked_rem(10).unwrap_or(0);
                format!("{whole}.{tenth}M")
            } else {
                format!("{}M", rounded(count, 1_000_000))
            }
        }
    }
}

impl Words {
    /// A duration in milliseconds, short: `40 ms`, `1.2 s`, `2 m 5 s`.
    #[must_use]
    pub fn duration(self, ms: u64) -> String {
        if ms < 1000 {
            return match self.lang {
                Lang::En => format!("{ms} ms"),
                Lang::Ko => format!("{ms}ms"),
            };
        }
        if ms < 60_000 {
            let tenths = tenths(ms, 1000);
            let whole = tenths.checked_div(10).unwrap_or(0);
            let tenth = tenths.checked_rem(10).unwrap_or(0);
            return match self.lang {
                Lang::En => format!("{whole}.{tenth} s"),
                Lang::Ko => format!("{whole}.{tenth}초"),
            };
        }
        let seconds = rounded(ms, 1000);
        let (minutes, rest) = (
            seconds.checked_div(60).unwrap_or(0),
            seconds.checked_rem(60).unwrap_or(0),
        );
        match self.lang {
            Lang::En => format!("{minutes} m {rest} s"),
            Lang::Ko => format!("{minutes}분 {rest}초"),
        }
    }

    /// A local time of day, or a date with it when not today: `today
    /// 14:05`, `2026-11-07 14:05`. `now` carries the local offset.
    #[must_use]
    pub fn when(self, at: DateTime<Utc>, now: DateTime<FixedOffset>) -> String {
        let local = at.with_timezone(&now.timezone());
        let time = local.format("%H:%M");
        if local.date_naive() == now.date_naive() {
            match self.lang {
                Lang::En => format!("today {time}"),
                Lang::Ko => format!("오늘 {time}"),
            }
        } else {
            format!("{} {time}", local.format("%Y-%m-%d"))
        }
    }
}
