//! Log counts are typed before a node or credential file is opened.

use clap::Args;
use vessel_proto::EventSeq;

use crate::log::tail::Tail;
use crate::selector::SessionSelector;

/// Snapshot log selection; later selectors share this parsed count.
#[derive(Debug, Args)]
pub(crate) struct Log {
    /// Session id, or a unique prefix of one.
    pub(crate) session: SessionSelector,
    /// First record considered, before tail selection.
    #[arg(long, default_value = "0", value_parser = super::parse_seq)]
    pub(crate) from: EventSeq,
    /// Last N history records; zero prints nothing, omission prints all.
    #[arg(long)]
    pub(crate) tail: Option<Tail>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{Cli, Command};
    use clap::Parser as _;

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
