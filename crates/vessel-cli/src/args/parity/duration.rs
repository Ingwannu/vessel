//! Finite wait budgets parse Paseo's compound seconds/minutes/hours/days grammar.

use std::{fmt, str::FromStr, time::Duration};

/// A positive representable deadline; absence uses a documented safety ceiling.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WaitBudget(Duration);

impl WaitBudget {
    /// Every wait remains finite even if Paseo's omitted timeout has no limit.
    pub(crate) const DEFAULT: Self = Self(Duration::from_secs(24 * 60 * 60));
    /// Seconds per supported suffix, named here to keep conversion checked.
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * Self::MINUTE;
    const DAY: u64 = 24 * Self::HOUR;
    pub(crate) const fn duration(self) -> Duration {
        self.0
    }
}

#[derive(Debug, thiserror::Error)]
#[error("timeout must be a positive duration (30s, 5m, 2h30m or seconds) that fits a deadline")]
pub(crate) struct InvalidDuration;

impl FromStr for WaitBudget {
    type Err = InvalidDuration;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let text = raw.trim();
        let mut digits = String::new();
        let mut total = 0_u64;
        for character in text.chars() {
            if character.is_ascii_digit() {
                digits.push(character);
                continue;
            }
            let scale = match character {
                's' => 1,
                'm' => Self::MINUTE,
                'h' => Self::HOUR,
                'd' => Self::DAY,
                _ => return Err(InvalidDuration),
            };
            let number = digits.parse::<u64>().map_err(|_invalid| InvalidDuration)?;
            total = total
                .checked_add(number.checked_mul(scale).ok_or(InvalidDuration)?)
                .ok_or(InvalidDuration)?;
            digits.clear();
        }
        if !digits.is_empty() {
            if total != 0 || !text.chars().all(|c| c.is_ascii_digit()) {
                return Err(InvalidDuration);
            }
            total = digits.parse().map_err(|_invalid| InvalidDuration)?;
        }
        let duration = Duration::from_secs(total);
        if duration.is_zero() || std::time::Instant::now().checked_add(duration).is_none() {
            return Err(InvalidDuration);
        }
        Ok(Self(duration))
    }
}
impl fmt::Display for WaitBudget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0.as_secs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compound_and_plain_durations_reject_partial_or_unbounded_values() {
        for (raw, seconds) in [("2h30m", 9000), (" 90 ", 90), ("1d", 86400), ("0h1s", 1)] {
            assert_eq!(
                raw.parse::<WaitBudget>().unwrap().duration().as_secs(),
                seconds
            );
        }
        for raw in [
            "",
            "0",
            "0s",
            "1.5s",
            "1h30",
            "s",
            "NaN",
            "999999999999999999999d",
            "1m-2s",
        ] {
            assert!(raw.parse::<WaitBudget>().is_err(), "{raw}");
        }
    }
}
