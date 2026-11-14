//! Shutdown choices are parsed here so command code never swaps booleans.
//! This module does not contact a node or read its persisted state.

use std::fmt;
use std::str::FromStr;
use std::time::{Duration, Instant};

use clap::Args;

/// The user's authority to interrupt sessions, separate from its timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Confirmation {
    /// Interactive approval is still required.
    Ask,
    /// The person explicitly supplied `--yes`.
    Granted,
}

/// Live sessions require an explicit timing choice before shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StopPolicy {
    /// List live sessions and refuse to interrupt them.
    Protect,
    /// Refuse new sessions and let existing sessions end.
    Drain,
    /// Interrupt sessions after the required confirmation.
    Interrupt(Confirmation),
}

/// A finite, positive shutdown deadline; invalid values cannot reach a call.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StopTimeout(Duration);

impl StopTimeout {
    /// Paseo's graceful-exit budget is also the CLI's one default.
    const DEFAULT: Self = Self(Duration::from_secs(15));
    /// Millisecond rounding has one representable positive lower bound.
    const MINIMUM: Duration = Duration::from_millis(1);
    /// Integer unit conversion avoids introducing floating-point arithmetic.
    const NANOS_PER_MILLISECOND: u32 = 1_000_000;

    /// Bound the entire graceful command, including connection and RPC waits.
    pub(crate) const fn duration(self) -> Duration {
        self.0
    }
}

impl fmt::Display for StopTimeout {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.as_secs_f64().fmt(formatter)
    }
}

impl FromStr for StopTimeout {
    type Err = InvalidStopTimeout;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let seconds: f64 = text.trim().parse().map_err(InvalidStopTimeout::Number)?;
        let duration = Duration::try_from_secs_f64(seconds).map_err(InvalidStopTimeout::Range)?;
        // Paseo rounds positive seconds up to a millisecond. Checked rounding
        // refuses an unrepresentable deadline rather than wrapping its unit.
        let fractional_millisecond = duration
            .subsec_nanos()
            .checked_rem(Self::NANOS_PER_MILLISECOND)
            .is_some_and(|remainder| remainder != 0);
        // Conversion to nanoseconds may round a positive fraction down to a
        // whole millisecond (including zero). Preserve that fraction for ceil.
        let rounded_down = duration.as_secs_f64().total_cmp(&seconds).is_lt();
        let extra = u128::from(fractional_millisecond || rounded_down);
        let millis = duration
            .as_millis()
            .checked_add(extra)
            .ok_or(InvalidStopTimeout::Deadline)?;
        let millis = u64::try_from(millis).map_err(InvalidStopTimeout::Millis)?;
        let duration = Duration::from_millis(millis);
        if duration < Self::MINIMUM || Instant::now().checked_add(duration).is_none() {
            return Err(InvalidStopTimeout::Deadline);
        }
        Ok(Self(duration))
    }
}

/// Retain parsing causes without echoing arbitrary command-line input.
#[derive(Debug, thiserror::Error)]
pub(crate) enum InvalidStopTimeout {
    /// The text is not a number of seconds.
    #[error("timeout must be positive seconds (minimum rounded deadline: {} ms)", StopTimeout::MINIMUM.as_millis())]
    Number(#[source] std::num::ParseFloatError),
    /// NaN, infinity and negative durations cannot be deadlines.
    #[error("timeout must be finite positive seconds (minimum rounded deadline: {} ms)", StopTimeout::MINIMUM.as_millis())]
    Range(#[source] std::time::TryFromFloatSecsError),
    /// The rounded milliseconds exceed the supported duration.
    #[error("timeout exceeds the supported deadline")]
    Millis(#[source] std::num::TryFromIntError),
    /// A zero or unrepresentable monotonic deadline.
    #[error("timeout must round to at least {} ms and fit a monotonic deadline", StopTimeout::MINIMUM.as_millis())]
    Deadline,
}

/// Command-line flags stay private at the decoding edge; commands get a policy.
#[derive(Debug, Args)]
pub(crate) struct Stop {
    /// Let live sessions finish; refuse new ones meanwhile.
    #[arg(long, conflicts_with = "now")]
    when_idle: bool,
    /// Interrupt live sessions after asking for confirmation.
    #[arg(long)]
    now: bool,
    #[command(flatten)]
    confirmation: ApproveStop,
    /// Graceful exit deadline in positive seconds.
    #[arg(long, default_value_t = StopTimeout::DEFAULT)]
    pub(crate) timeout: StopTimeout,
}

/// Keep the confirmation flag separate from the live-session timing flags.
#[derive(Debug, Args)]
struct ApproveStop {
    /// Confirm interruption without an interactive terminal.
    #[arg(long, requires = "now")]
    yes: bool,
}

impl Stop {
    /// Carry the parsed intent across the command boundary as one choice.
    pub(crate) const fn policy(&self) -> StopPolicy {
        if self.when_idle {
            StopPolicy::Drain
        } else if self.now {
            let approval = if self.confirmation.yes {
                Confirmation::Granted
            } else {
                Confirmation::Ask
            };
            StopPolicy::Interrupt(approval)
        } else {
            StopPolicy::Protect
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{Cli, Command, NodeCommand};
    use clap::Parser as _;

    #[test]
    fn deadlines_follow_paseos_positive_seconds_and_millisecond_rounding() {
        let below_minimum = StopTimeout::MINIMUM.saturating_sub(StopTimeout::MINIMUM);
        let invalid = [
            below_minimum.as_secs_f64().to_string(),
            format!("-{}", StopTimeout::MINIMUM.as_secs_f64()),
            f64::MAX.to_string(),
            "NaN".to_owned(),
            "inf".to_owned(),
            "word".to_owned(),
        ];
        for text in invalid {
            let error = text.parse::<StopTimeout>().unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains(&format!(" {} ms", StopTimeout::MINIMUM.as_millis()))
            );
        }
        assert_eq!(
            "0.0011".parse::<StopTimeout>().unwrap().duration(),
            Duration::from_millis(2)
        );
        assert_eq!(
            "1e-50".parse::<StopTimeout>().unwrap().duration(),
            StopTimeout::MINIMUM
        );
        assert_eq!(
            "1.00000000001".parse::<StopTimeout>().unwrap().duration(),
            Duration::from_millis(1001)
        );
        assert_eq!(
            StopTimeout::DEFAULT
                .to_string()
                .parse::<StopTimeout>()
                .unwrap()
                .duration(),
            StopTimeout::DEFAULT.duration()
        );
    }

    #[test]
    fn cli_parses_policy_and_rejects_conflicts_before_contact() {
        for (flags, policy) in [
            (vec![], StopPolicy::Protect),
            (vec!["--when-idle"], StopPolicy::Drain),
            (vec!["--now"], StopPolicy::Interrupt(Confirmation::Ask)),
            (
                vec!["--now", "--yes"],
                StopPolicy::Interrupt(Confirmation::Granted),
            ),
        ] {
            let args = [vec!["vessel", "node", "stop"], flags].concat();
            let cli = Cli::try_parse_from(args).unwrap();
            let Command::Node {
                command: NodeCommand::Stop(stop),
            } = cli.command
            else {
                panic!("wrong command");
            };
            assert_eq!(stop.policy(), policy);
            assert_eq!(stop.timeout.duration(), StopTimeout::DEFAULT.duration());
        }
        let below_minimum = StopTimeout::MINIMUM
            .saturating_sub(StopTimeout::MINIMUM)
            .as_secs_f64()
            .to_string();
        for args in [
            vec!["--now", "--when-idle"],
            vec!["--yes"],
            vec!["--timeout", &below_minimum],
        ] {
            let args = [vec!["vessel", "node", "stop"], args].concat();
            assert!(Cli::try_parse_from(args).is_err());
        }
    }
}
