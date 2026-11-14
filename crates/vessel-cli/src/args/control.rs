//! Shutdown choices are parsed here so command code never swaps booleans.
//! This module does not contact a node or read its persisted state.

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
    /// Bound the entire graceful command, including connection and RPC waits.
    pub(crate) const fn duration(self) -> Duration {
        self.0
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
            .checked_rem(1_000_000)
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
        if duration.is_zero() || Instant::now().checked_add(duration).is_none() {
            return Err(InvalidStopTimeout::Deadline);
        }
        Ok(Self(duration))
    }
}

/// Retain parsing causes without echoing arbitrary command-line input.
#[derive(Debug, thiserror::Error)]
pub(crate) enum InvalidStopTimeout {
    /// The text is not a number of seconds.
    #[error("timeout must be a positive number of seconds")]
    Number(#[source] std::num::ParseFloatError),
    /// NaN, infinity and negative durations cannot be deadlines.
    #[error("timeout must be a finite positive number of seconds")]
    Range(#[source] std::time::TryFromFloatSecsError),
    /// The rounded milliseconds exceed the supported duration.
    #[error("timeout exceeds the supported deadline")]
    Millis(#[source] std::num::TryFromIntError),
    /// A zero or unrepresentable monotonic deadline.
    #[error("timeout must be positive and fit a monotonic deadline")]
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
    /// Graceful exit deadline in positive seconds (default: 15).
    #[arg(long, default_value = "15")]
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
        for text in ["0", "-1", "NaN", "inf", "1e300", "word"] {
            assert!(text.parse::<StopTimeout>().is_err());
        }
        assert_eq!(
            "0.0011".parse::<StopTimeout>().unwrap().duration(),
            Duration::from_millis(2)
        );
        assert_eq!(
            "1e-50".parse::<StopTimeout>().unwrap().duration(),
            Duration::from_millis(1)
        );
        assert_eq!(
            "1.00000000001".parse::<StopTimeout>().unwrap().duration(),
            Duration::from_millis(1001)
        );
        assert_eq!(
            "15".parse::<StopTimeout>().unwrap().duration(),
            Duration::from_secs(15)
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
        }
        for args in [
            vec!["--now", "--when-idle"],
            vec!["--yes"],
            vec!["--timeout", "0"],
        ] {
            let args = [vec!["vessel", "node", "stop"], args].concat();
            assert!(Cli::try_parse_from(args).is_err());
        }
    }
}
