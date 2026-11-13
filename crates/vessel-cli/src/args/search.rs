//! `vessel search` (protocol 12): every session's messages.

use clap::Args;
use vessel_proto::search::{SearchCursor, SearchLimit, SearchQuery};

use crate::selector::SessionSelector;

/// `vessel search`'s arguments.
#[derive(Debug, Args)]
pub(crate) struct Search {
    /// The words; every one must be in a message (in any order, inside
    /// other words, without case).
    #[arg(required = true, num_args = 1..)]
    pub(crate) words: Vec<String>,
    /// Only this session's messages (its id, or a unique prefix of one).
    #[arg(long)]
    pub(crate) session: Option<SessionSelector>,
    /// At most this many hits (1 to 100).
    #[arg(long, default_value = "20", value_parser = parse_limit)]
    pub(crate) limit: SearchLimit,
    /// Go on from an earlier search's last line (`--before <cursor>`).
    #[arg(long)]
    pub(crate) before: Option<SearchCursor>,
}

impl Search {
    /// The words as one query.
    pub(crate) fn query(&self) -> Result<SearchQuery, vessel_proto::search::InvalidSearch> {
        self.words.join(" ").parse()
    }
}

fn parse_limit(text: &str) -> Result<SearchLimit, String> {
    text.parse::<u8>()
        .ok()
        .and_then(SearchLimit::new)
        .ok_or_else(|| "a number from 1 to 100".to_owned())
}
