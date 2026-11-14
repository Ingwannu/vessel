//! Log inputs are typed before a node or credential file is opened.

use clap::{
    Args,
    builder::{BoolishValueParser, TypedValueParser as _},
};
use vessel_proto::EventSeq;

use crate::log::select::{Filter, Mode, Selection, Since};
use crate::log::tail::Tail;
use crate::selector::SessionSelector;

/// History selection followed optionally by the existing live stream.
#[derive(Debug, Args)]
pub(crate) struct Log {
    /// Session id, or a unique prefix of one.
    pub(crate) session: SessionSelector,
    /// First record considered, before time, kind and tail selection.
    #[arg(long, default_value = "0", value_parser = super::parse_seq)]
    pub(crate) from: EventSeq,
    /// Continue with future matching records until the session ends or Ctrl-C.
    #[arg(long = "follow", short = 'f', action = clap::ArgAction::SetTrue,
        value_parser = BoolishValueParser::new().map(|enabled| if enabled { Mode::Follow } else { Mode::Once }))]
    pub(crate) mode: Mode,
    /// Last N matching history records; zero prints no history. Follow uses its default history window.
    #[arg(long, help = format!("Last N matching history records; zero prints no history. Follow defaults to {}.", Tail::FOLLOW_DEFAULT))]
    pub(crate) tail: Option<Tail>,
    /// Keep only one of the named event kinds.
    #[arg(long, value_enum, ignore_case = true)]
    pub(crate) filter: Option<Filter>,
    /// Keep records at or after this RFC 3339 timestamp, including its timezone.
    #[arg(long)]
    pub(crate) since: Option<Since>,
}

impl Log {
    /// Pass only parsed predicates to the transport loop.
    pub(crate) fn selection(&self) -> Selection {
        Selection {
            since: self.since,
            filter: self.filter,
        }
    }

    /// Only an omitted follow limit inherits the shared default.
    pub(crate) fn history_tail(&self) -> Option<Tail> {
        self.tail.or(match self.mode {
            Mode::Once => None,
            Mode::Follow => Some(Tail::FOLLOW_DEFAULT),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{Cli, Command};
    use clap::Parser as _;

    fn parse(flags: &[&str]) -> Log {
        let parsed = Cli::try_parse_from(
            ["vessel", "log", "abcd"]
                .into_iter()
                .chain(flags.iter().copied()),
        )
        .unwrap();
        let Command::Log(log) = parsed.command else {
            panic!("expected log")
        };
        log
    }

    #[test]
    fn plain_history_has_no_tail_and_follow_uses_its_single_default() {
        let once = parse(&[]);
        assert_eq!(once.mode, Mode::Once);
        assert_eq!(once.history_tail(), None);
        for flag in ["-f", "--follow"] {
            let following = parse(&[flag]);
            assert_eq!(following.mode, Mode::Follow);
            assert_eq!(following.history_tail(), Some(Tail::FOLLOW_DEFAULT));
        }
        assert_eq!(
            parse(&["--follow", "--tail", "0"]).history_tail(),
            Some(Tail::ZERO)
        );
        assert_eq!(
            parse(&["--tail", "2"]).history_tail(),
            Some("2".parse().unwrap())
        );
    }

    #[test]
    fn invalid_selectors_are_usage_errors_and_never_reach_connection_work() {
        for flags in [
            ["--filter", "typo"],
            ["--since", "yesterday"],
            ["--since", "2026-01-01T00:00:00"],
        ] {
            assert!(
                Cli::try_parse_from(["vessel", "log", "abcd"].into_iter().chain(flags)).is_err(),
                "{flags:?}"
            );
        }
        let parsed = parse(&[
            "--from",
            "7",
            "--since",
            "2026-01-01T01:00:00+01:00",
            "--filter",
            "TEXT",
        ]);
        assert_eq!(parsed.from, EventSeq::new(7));
        assert_eq!(parsed.filter, Some(Filter::Text));
        assert!(parsed.since.is_some());
    }
    #[test]
    fn a_tail_is_optional_or_a_whole_nonnegative_count_before_connection_work() {
        for (flags, expected) in [
            (Vec::new(), None),
            (vec!["--tail", "0"], Some(Tail::ZERO)),
            (vec!["--tail", "2"], Some("2".parse().unwrap())),
        ] {
            let parsed =
                Cli::try_parse_from(["vessel", "log", "abcd"].into_iter().chain(flags)).unwrap();
            let Command::Log(log) = parsed.command else {
                panic!("expected log")
            };
            assert_eq!(log.tail, expected);
        }
        for raw in [
            "-1",
            "abc",
            "2x",
            "1.5",
            "9999999999999999999999999999999999999",
        ] {
            assert!(
                Cli::try_parse_from(["vessel", "log", "abcd", "--tail", raw]).is_err(),
                "{raw}"
            );
        }
    }
}
