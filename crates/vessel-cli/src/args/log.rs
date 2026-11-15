//! Log inputs are typed before a node or credential file is opened.

use clap::Args;
use vessel_proto::EventSeq;

use crate::log::select::{Filter, Selection, Since};
use crate::log::tail::Tail;
use crate::selector::SessionSelector;

/// Snapshot selection with all inputs parsed before connection work.
#[derive(Debug, Args)]
pub(crate) struct Log {
    /// Session id, or a unique prefix of one.
    pub(crate) session: SessionSelector,
    /// First record considered, before time, kind and tail selection.
    #[arg(long, default_value = "0", value_parser = super::parse_seq)]
    pub(crate) from: EventSeq,
    /// Last N matching history records; zero prints no history.
    #[arg(long)]
    pub(crate) tail: Option<Tail>,
    /// Keep only one of the named event kinds.
    #[arg(long, value_enum, ignore_case = true)]
    pub(crate) filter: Option<Filter>,
    /// Keep records at or after this RFC 3339 timestamp, including its timezone.
    /// Raw records, like --from and --tail, can begin inside a message or tool lifecycle.
    #[arg(long)]
    pub(crate) since: Option<Since>,
}

impl Log {
    /// Pass only parsed predicates to the transport loop.
    #[must_use]
    pub(crate) fn selection(&self) -> Selection {
        Selection {
            since: self.since,
            filter: self.filter,
        }
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
        let overflow = format!("{}0", usize::MAX);
        for raw in ["-1", "abc", "2x", "1.5", overflow.as_str()] {
            assert!(
                Cli::try_parse_from(["vessel", "log", "abcd", "--tail", raw]).is_err(),
                "{raw}"
            );
        }
    }
}
